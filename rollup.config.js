import rust from "@wasm-tool/rollup-plugin-rust";
import { terser } from "rollup-plugin-terser";

let items = [];
items.push({
  input: {
    "index-iife": "index.js",
  },
  output: {
    format: "iife",
    name: "Graphima",
    dir: "dist",
  },
  plugins: [
    rust({
      inlineWasm: true,
      wasmOptArgs: ["-Oz"],
    }),
    terser(),
  ],
});
if (process.env.BUILD_ALL) {
  items.push({
    input: {
      "index-esm": "index.js",
    },
    output: {
      format: "es",
      dir: "dist",
      esModule: true,
    },
    plugins: [
      rust({
      inlineWasm: true,
      wasmOptArgs: ["-Oz"],
      }),
      terser(),
    ],
  });
}

export default items;
