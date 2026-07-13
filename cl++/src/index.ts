import fs from "fs";
import path from "path";
import peggy from "peggy";
import exitWithError from "./utils/exitWithError.js";

const main = () => {
  const filePath = process.argv[2];

  if (!filePath) exitWithError("Error: path to .clx file not specified");

  if (!fs.existsSync(filePath))
    exitWithError(`Error: not found file: ${filePath}`);

  try {
    const sourceCode = fs.readFileSync(filePath, "utf-8");

    const grammarPath = path.resolve("src/grammar.peggy");
    const grammar = fs.readFileSync(grammarPath, "utf-8");

    const parser = peggy.generate(grammar);

    const ast: string = parser.parse(sourceCode);

    console.log(JSON.stringify(ast, null, 2));

    console.log(sourceCode);
  } catch (error: unknown) {
    exitWithError(`Error: fail to read file: ${filePath}`, error);
  }
};

main();
