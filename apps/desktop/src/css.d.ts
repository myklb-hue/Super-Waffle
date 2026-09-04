// CSS Modules are resolved by the bundler; this tells tsc what the default
// export looks like. Values are `string | undefined` under
// noUncheckedIndexedAccess, which React's className accepts.
declare module '*.module.css' {
  const classes: Readonly<Record<string, string>>;
  export default classes;
}
