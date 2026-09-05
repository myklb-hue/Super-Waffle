#!/usr/bin/env python3
"""Cyberloom's perception helper: the Python half of `crates/loomd/src/run/perceive.rs`.

The engine runs this as it would a custom block — one process per question,
arguments as JSON, the answer in a file:

    perceive.py <task> '<json request>' <out.json>

Seven tasks, one per method of the `Perception` trait, each answered with the
shape the Rust side deserialises:

    detect      {image, model}           -> [{label, confidence, box: [x, y, w, h]}]
    recognise   {image, threshold}       -> {name, confidence, dimensions}
    transcribe  {audio, model}           -> {text, seconds}
    speak       {text, voice, into}      -> writes a WAV to `into`
    classify    {text, labels}           -> {label, confidence}
    affect      {text}                   -> {valence, arousal}
    embed       {text, model}            -> [float, ...]

Everything it needs lives beside it, in the models folder it is installed to:
`yolov8n.onnx`, `voices/`, `whisper/`, `insightface/`, `fastembed/`, and the
enrolled `people/` (embeddings only — SPEC §12.3). `scripts/provision.sh` puts
all of that in place and creates the virtual environment this runs in.

Imports are per task and lazy, so a package that is missing breaks only the
task that needs it, and breaks it with a `ModuleNotFoundError` on the last line
of stderr, which is what the engine reads to say "not installed" rather than
"broken".
"""
from __future__ import annotations

import json
import math
import sys
import wave
from pathlib import Path

HERE = Path(__file__).resolve().parent

PROVISION = "run scripts/provision.sh (or cyberloom-provision) to set up perception"


def fail(message: str) -> None:
    print(message, file=sys.stderr)
    sys.exit(1)


def providers() -> list[str]:
    """ONNX Runtime on the GPU when there is one, the CPU otherwise."""
    try:
        import onnxruntime as ort

        have = ort.get_available_providers()
        return [p for p in ("CUDAExecutionProvider", "ROCMExecutionProvider", "CPUExecutionProvider") if p in have]
    except ImportError:
        return ["CPUExecutionProvider"]


def on_gpu() -> bool:
    return providers()[0] != "CPUExecutionProvider"


# ------------------------------------------------------------------ detect

COCO = (
    "person bicycle car motorcycle airplane bus train truck boat traffic-light fire-hydrant stop-sign "
    "parking-meter bench bird cat dog horse sheep cow elephant bear zebra giraffe backpack umbrella handbag "
    "tie suitcase frisbee skis snowboard sports-ball kite baseball-bat baseball-glove skateboard surfboard "
    "tennis-racket bottle wine-glass cup fork knife spoon bowl banana apple sandwich orange broccoli carrot "
    "hot-dog pizza donut cake chair couch potted-plant bed dining-table toilet tv laptop mouse remote keyboard "
    "cell-phone microwave oven toaster sink refrigerator book clock vase scissors teddy-bear hair-drier toothbrush"
).split()


def detect(req: dict) -> list[dict]:
    """YOLOv8 through ONNX Runtime: letterbox in, boxes in image pixels out."""
    import numpy as np
    import onnxruntime as ort
    from PIL import Image

    model = req.get("model") or "yolov8n"
    weights = HERE / f"{model}.onnx"
    if not weights.exists():
        fail(f"No such file: {weights} — {PROVISION}")

    image = Image.open(req["image"]).convert("RGB")
    width, height = image.size
    side = 640
    scale = min(side / width, side / height)
    new_w, new_h = max(1, round(width * scale)), max(1, round(height * scale))
    pad_x, pad_y = (side - new_w) // 2, (side - new_h) // 2
    canvas = Image.new("RGB", (side, side), (114, 114, 114))
    canvas.paste(image.resize((new_w, new_h)), (pad_x, pad_y))
    x = np.asarray(canvas, dtype=np.float32).transpose(2, 0, 1)[None] / 255.0

    session = ort.InferenceSession(str(weights), providers=providers())
    out = session.run(None, {session.get_inputs()[0].name: x})[0]
    preds = out[0].T  # (anchors, 4 + classes)
    scores = preds[:, 4:]
    classes = scores.argmax(1)
    confidence = scores.max(1)
    keep = confidence > 0.25
    boxes, classes, confidence = preds[keep, :4], classes[keep], confidence[keep]
    if boxes.size == 0:
        return []
    # centre/size in the letterbox -> corners in the image.
    x1 = (boxes[:, 0] - boxes[:, 2] / 2 - pad_x) / scale
    y1 = (boxes[:, 1] - boxes[:, 3] / 2 - pad_y) / scale
    x2 = (boxes[:, 0] + boxes[:, 2] / 2 - pad_x) / scale
    y2 = (boxes[:, 1] + boxes[:, 3] / 2 - pad_y) / scale
    x1, y1 = np.clip(x1, 0, width), np.clip(y1, 0, height)
    x2, y2 = np.clip(x2, 0, width), np.clip(y2, 0, height)

    seen = []
    for cls in np.unique(classes):
        idx = np.where(classes == cls)[0]
        order = idx[np.argsort(-confidence[idx])]
        while order.size:
            i = order[0]
            seen.append(
                {
                    "label": COCO[int(cls)] if int(cls) < len(COCO) else str(int(cls)),
                    "confidence": float(confidence[i]),
                    "box": [float(x1[i]), float(y1[i]), float(x2[i] - x1[i]), float(y2[i] - y1[i])],
                }
            )
            rest = order[1:]
            if rest.size == 0:
                break
            ix1, iy1 = np.maximum(x1[i], x1[rest]), np.maximum(y1[i], y1[rest])
            ix2, iy2 = np.minimum(x2[i], x2[rest]), np.minimum(y2[i], y2[rest])
            inter = np.clip(ix2 - ix1, 0, None) * np.clip(iy2 - iy1, 0, None)
            area_i = (x2[i] - x1[i]) * (y2[i] - y1[i])
            area_r = (x2[rest] - x1[rest]) * (y2[rest] - y1[rest])
            iou = inter / np.maximum(area_i + area_r - inter, 1e-9)
            order = rest[iou < 0.45]
    seen.sort(key=lambda s: -s["confidence"])
    return seen


# --------------------------------------------------------------- recognise


def recognise(req: dict) -> dict:
    """Who is in the frame, as an embedding matched against the enrolled.

    Never an image: what is kept per person is a vector (SPEC §12.3). The
    enrolled live in `people/<name>.npy`, one normalised embedding each.
    """
    import cv2
    import numpy as np
    from insightface.app import FaceAnalysis

    app = FaceAnalysis(name="buffalo_l", root=str(HERE / "insightface"), providers=providers())
    app.prepare(ctx_id=0 if on_gpu() else -1, det_size=(640, 640))
    frame = cv2.imread(req["image"])
    if frame is None:
        fail(f"could not read {req['image']}")
    faces = app.get(frame)
    if not faces:
        return {"name": None, "confidence": 0.0, "dimensions": 512}
    face = max(faces, key=lambda f: (f.bbox[2] - f.bbox[0]) * (f.bbox[3] - f.bbox[1]))
    embedding = np.asarray(face.normed_embedding, dtype=np.float32)

    threshold = float(req.get("threshold") or 0.5)
    best_name, best = None, 0.0
    people = HERE / "people"
    if people.is_dir():
        for path in people.glob("*.npy"):
            known = np.load(path).astype(np.float32).ravel()
            if known.size != embedding.size:
                continue
            sim = float(np.dot(known, embedding) / (np.linalg.norm(known) * np.linalg.norm(embedding) + 1e-9))
            if sim > best:
                best_name, best = path.stem, sim
    matched = best_name if best >= threshold else None
    return {
        "name": matched,
        "confidence": best if matched else float(face.det_score),
        "dimensions": int(embedding.size),
    }


# -------------------------------------------------------------- transcribe

WHISPER_SIZES = {
    "whisper-tiny": "tiny",
    "whisper-base": "base",
    "whisper-small": "small",
    "whisper-medium": "medium",
    "whisper-large": "large-v3",
}


def transcribe(req: dict) -> dict:
    from faster_whisper import WhisperModel

    name = req.get("model") or "whisper-small"
    size = WHISPER_SIZES.get(name, name.removeprefix("whisper-"))
    gpu = False
    try:
        import ctranslate2

        gpu = ctranslate2.get_cuda_device_count() > 0
    except Exception:  # noqa: BLE001 - no CUDA is not an error
        gpu = False
    model = WhisperModel(
        size,
        device="cuda" if gpu else "cpu",
        compute_type="float16" if gpu else "int8",
        download_root=str(HERE / "whisper"),
    )
    segments, info = model.transcribe(req["audio"], vad_filter=True)
    text = " ".join(segment.text.strip() for segment in segments).strip()
    return {"text": text, "seconds": float(info.duration or 0.0)}


# ------------------------------------------------------------------- speak


def speak(req: dict) -> dict:
    from piper import PiperVoice

    voice_name = req.get("voice") or "en_GB-alan-medium"
    onnx = HERE / "voices" / f"{voice_name}.onnx"
    if not onnx.exists():
        fail(f"No such file: {onnx} — {PROVISION}")
    voice = PiperVoice.load(str(onnx), use_cuda=on_gpu())
    with wave.open(req["into"], "wb") as out:
        if hasattr(voice, "synthesize_wav"):
            voice.synthesize_wav(req["text"], out)
        else:  # piper-tts before 1.3
            voice.synthesize(req["text"], out)
    return {"ok": True}


# --------------------------------------------------------- classify, embed


def embedder():
    from fastembed import TextEmbedding

    return TextEmbedding(model_name="BAAI/bge-small-en-v1.5", cache_dir=str(HERE / "fastembed"))


def embed(req: dict) -> list[float]:
    vector = next(iter(embedder().embed([req["text"]])))
    return [float(v) for v in vector]


def classify(req: dict) -> dict:
    """Zero-shot, by similarity of the text to each label's own embedding."""
    import numpy as np

    labels = [str(label) for label in req.get("labels") or []]
    if not labels:
        fail("classify needs labels")
    vectors = [np.asarray(v, dtype=np.float32) for v in embedder().embed([req["text"], *labels])]
    text, options = vectors[0], vectors[1:]
    sims = np.array([float(np.dot(text, o) / (np.linalg.norm(text) * np.linalg.norm(o) + 1e-9)) for o in options])
    # A sharpened softmax, so one clear winner reads as confident and a tie as a coin.
    weights = np.exp((sims - sims.max()) * 20.0)
    confidence = weights / weights.sum()
    best = int(sims.argmax())
    return {"label": labels[best], "confidence": float(confidence[best])}


# ------------------------------------------------------------------ affect


def affect(req: dict) -> dict:
    """Valence from a sentiment lexicon; arousal from how loudly it is said."""
    from vaderSentiment.vaderSentiment import SentimentIntensityAnalyzer

    text = req.get("text") or ""
    scores = SentimentIntensityAnalyzer().polarity_scores(text)
    valence = float(scores["compound"])
    loaded = float(scores["pos"] + scores["neg"])
    letters = [c for c in text if c.isalpha()]
    shouting = sum(1 for c in letters if c.isupper()) / len(letters) if letters else 0.0
    bangs = min(text.count("!"), 3) / 3.0
    arousal = min(1.0, 0.45 * abs(valence) + 0.3 * min(1.0, loaded * 2.0) + 0.15 * bangs + 0.1 * (1.0 if shouting > 0.5 else 0.0))
    return {"valence": valence, "arousal": float(arousal)}


TASKS = {
    "detect": detect,
    "recognise": recognise,
    "transcribe": transcribe,
    "speak": speak,
    "classify": classify,
    "affect": affect,
    "embed": embed,
}


def main(argv: list[str]) -> None:
    if len(argv) != 4:
        fail("usage: perceive.py <task> '<json>' <out.json>")
    task, request, out = argv[1], json.loads(argv[2]), argv[3]
    run = TASKS.get(task)
    if run is None:
        fail(f"no such task: {task}")
    try:
        result = run(request)
    except ModuleNotFoundError as missing:
        # The engine reads the last line: keep the exception's name on it, and
        # say what to do.
        fail(f"ModuleNotFoundError: {missing} — {PROVISION}")
    Path(out).write_text(json.dumps(result))


if __name__ == "__main__":
    main(sys.argv)
