#!/usr/bin/env bash
# Put in place everything a first run of Cyberloom needs beyond the binary, so
# that the assistant graph runs the first time the window opens (SPEC §15.13:
# the network is allowed on first run; downloads are explicit and visible).
#
#   scripts/provision.sh            everything below
#   scripts/provision.sh --only X   one part: ollama | python | models
#
# What it does, in order, and where it puts things:
#
#   1. Ollama   the service enabled and started, and the default model
#               (llama3.2:3b) pulled. Nothing to install here: pacman did.
#   2. Python   a virtual environment at ~/.local/share/cyberloom/venv with
#               crates/loomd/py/requirements.txt in it, and perceive.py — the
#               engine's perception helper — installed into the models folder,
#               which is where the engine looks for it.
#   3. Models   the weights each perception block needs, into
#               ~/.local/share/cyberloom/models: the YOLOv8n detector (fetched
#               as .pt from GitHub and exported to ONNX once), the Piper voice,
#               the Whisper model, the insightface face pack, the embedding
#               model. Each is a download; each is skipped when it is there.
#
# Every step runs even if an earlier one failed, the failures are listed at the
# end, and the exit code says whether there were any — a network hiccup on one
# download should not leave the other four undone. Run it again to finish.
#
# The install script calls this; the AUR package installs it as
# `cyberloom-provision`; the application's settings screen names it when
# something is missing.
set -uo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
data="${XDG_DATA_HOME:-$HOME/.local/share}/cyberloom"
models="${CYBERLOOM_MODELS:-$data/models}"
venv="$data/venv"
model_name="${CYBERLOOM_MODEL:-llama3.2:3b}"
voice="${CYBERLOOM_VOICE:-en_GB-alan-medium}"
only="all"
case "${1:-}" in
  --only) only="${2:?--only needs ollama, python or models}" ;;
  -h|--help) sed -n '2,32p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'; exit 0 ;;
esac

say() { printf '\033[1m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
failed=()
# Run a step, remember if it failed, and carry on.
step() {
  local name="$1"; shift
  say "$name"
  if "$@"; then return 0; fi
  warn "$name did not finish"
  failed+=("$name")
  return 1
}
want() { [ "$only" = all ] || [ "$only" = "$1" ]; }

# The helper's source: a checkout, or where the package put it.
py_src=""
for candidate in "$here/../crates/loomd/py" "/usr/lib/cyberloom/py" "$here/py"; do
  if [ -f "$candidate/perceive.py" ]; then py_src="$(cd "$candidate" && pwd)"; break; fi
done

# ------------------------------------------------------------------ ollama
ollama_up() { curl -sf --max-time 2 http://127.0.0.1:11434/api/tags >/dev/null; }

start_ollama() {
  command -v ollama >/dev/null || { warn "ollama is not installed (pacman -S ollama, or ollama-cuda / ollama-rocm for a GPU)"; return 1; }
  if ! ollama_up; then
    if command -v systemctl >/dev/null && systemctl list-unit-files ollama.service >/dev/null 2>&1; then
      sudo systemctl enable --now ollama.service || return 1
    else
      warn "no ollama.service here; start it yourself with \`ollama serve\`"
    fi
  fi
  for _ in $(seq 1 30); do ollama_up && return 0; sleep 1; done
  warn "nothing is answering at http://127.0.0.1:11434"
  return 1
}

pull_model() {
  if ollama list 2>/dev/null | awk '{print $1}' | grep -qx "$model_name"; then
    echo "    $model_name is already pulled"
    return 0
  fi
  ollama pull "$model_name"
}

if want ollama; then
  step "Ollama: service" start_ollama && step "Ollama: pull $model_name" pull_model
fi

# ------------------------------------------------------------------ python
make_venv() {
  command -v python3 >/dev/null || { warn "python3 is not installed (pacman -S python)"; return 1; }
  [ -x "$venv/bin/python" ] || python3 -m venv "$venv" || return 1
  "$venv/bin/python" -m pip install --quiet --upgrade pip wheel
}

install_requirements() {
  [ -n "$py_src" ] || { warn "perceive.py not found beside this script or in /usr/lib/cyberloom/py"; return 1; }
  # torch is only here so ultralytics can export the detector once; the CPU
  # wheel is a fraction of the CUDA one and does that job just as well.
  "$venv/bin/python" -c "import torch" 2>/dev/null || \
    "$venv/bin/python" -m pip install --quiet torch --index-url https://download.pytorch.org/whl/cpu || return 1
  "$venv/bin/python" -m pip install --quiet -r "$py_src/requirements.txt"
}

install_helper() {
  [ -n "$py_src" ] || return 1
  install -Dm644 "$py_src/perceive.py" "$models/perceive.py" && mkdir -p "$models/people"
}

if want python; then
  step "Python: virtual environment at $venv" make_venv \
    && step "Python: packages (this is the long one)" install_requirements
  step "Python: perception helper into $models" install_helper
fi

# ------------------------------------------------------------------ models
py() { "$venv/bin/python" "$@"; }

detector() {
  [ -f "$models/yolov8n.onnx" ] && { echo "    yolov8n.onnx is here"; return 0; }
  mkdir -p "$models"
  [ -f "$models/yolov8n.pt" ] || curl -fL --retry 3 -o "$models/yolov8n.pt" \
    https://github.com/ultralytics/assets/releases/download/v8.3.0/yolov8n.pt || return 1
  (cd "$models" && py -c "from ultralytics import YOLO; YOLO('yolov8n.pt').export(format='onnx', imgsz=640, opset=12)") >/dev/null || return 1
  [ -f "$models/yolov8n.onnx" ]
}

voice_files() {
  [ -f "$models/voices/$voice.onnx" ] && { echo "    $voice is here"; return 0; }
  mkdir -p "$models/voices"
  py -m piper.download_voices "$voice" --data-dir "$models/voices" 2>/dev/null && [ -f "$models/voices/$voice.onnx" ] && return 0
  # Older piper-tts has no downloader; the voices live on Hugging Face.
  local lang="${voice%%-*}" rest="${voice#*-}" name quality
  name="${rest%-*}"; quality="${rest##*-}"
  local base="https://huggingface.co/rhasspy/piper-voices/resolve/main/${lang%%_*}/$lang/$name/$quality"
  curl -fL --retry 3 -o "$models/voices/$voice.onnx" "$base/$voice.onnx" \
    && curl -fL --retry 3 -o "$models/voices/$voice.onnx.json" "$base/$voice.onnx.json"
}

whisper_model() {
  py -c "from faster_whisper import WhisperModel; WhisperModel('small', device='cpu', compute_type='int8', download_root='$models/whisper')" >/dev/null
}

face_pack() {
  py -c "from insightface.app import FaceAnalysis; FaceAnalysis(name='buffalo_l', root='$models/insightface', providers=['CPUExecutionProvider']).prepare(ctx_id=-1)" >/dev/null 2>&1
}

embedding_model() {
  py -c "from fastembed import TextEmbedding; TextEmbedding(model_name='BAAI/bge-small-en-v1.5', cache_dir='$models/fastembed')" >/dev/null
}

check_helper() {
  local out; out="$(mktemp)"
  py "$models/perceive.py" affect '{"text":"what a lovely morning"}' "$out" && cat "$out" && echo && rm -f "$out"
}

if want models; then
  if [ -x "$venv/bin/python" ]; then
    step "Models: YOLOv8n detector" detector
    step "Models: Piper voice $voice" voice_files
    step "Models: Whisper small" whisper_model
    step "Models: insightface faces" face_pack
    step "Models: sentence embeddings" embedding_model
    step "Models: the helper answers" check_helper
  else
    warn "no virtual environment at $venv; run the python part first"
    failed+=("Models")
  fi
fi

# ------------------------------------------------------------------ report
if [ ${#failed[@]} -eq 0 ]; then
  say "Everything a first run needs is in place."
  exit 0
fi
warn "not finished: ${failed[*]}"
echo "    Run this again once the network is back; what is already here is skipped." >&2
exit 1
