import { defineConfig } from "tsup";
import { copy } from "esbuild-plugin-copy";

export default defineConfig({
  entry: ["src/index.ts"],
  outDir: "dist",
  loader: {
    ".peggy": "file",
  },
  clean: true,
  format: "esm",
  minify: true,
  esbuildPlugins: [
    copy({
      assets: {
        from: ["./src/grammar.peggy"],
        to: ["."],
      },
    }),
  ],
});
