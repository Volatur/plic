import fs from "fs";
import peggy from "peggy";
import path from "path";
import { fileURLToPath } from "url";
import exitWithError from "./utils/exitWithError.js";
import { type ASTNode, type ImportDeclaration } from "./types/index.js";
import SemanticAnalyzer from "./semantic.js";
import generate from "./codegen.js";

const __filename = fileURLToPath(import.meta.url)
const __dirname = path.dirname(__filename)

const grammartPath = path.resolve(__dirname, "grammar.peggy")
const grammar = fs.readFileSync(grammartPath, "utf-8")
const parser = peggy.generate(grammar)


const compiledFiles = new Set<string>()

const compileModule = (filePath: string) => {
    const absolutePath = path.resolve(filePath)

  if (compiledFiles.has(absolutePath)) return
  compiledFiles.add(absolutePath)

  if (!fs.existsSync(absolutePath)) exitWithError(`File not found: ${absolutePath}`)

  try {
    const sourceCode = fs.readFileSync(absolutePath, "utf-8")
    const ast: ASTNode = parser.parse(sourceCode)

    if (ast.type === "Program") {
      const imports = ast.body.filter((node): node is ImportDeclaration => node.type === "ImportDeclaration")

      for (const imp of imports) {
        const importPath = path.join(path.dirname(absolutePath), imp.source.value + ".clx")
        compileModule(importPath)
      }
    }

    const analyzer = new SemanticAnalyzer()
    analyzer.analyze(ast)

    const moduleName = path.basename(absolutePath, ".clx")
    const code = generate(ast, moduleName)

    const outputPath = path.join(path.dirname(absolutePath), `${moduleName}.erl`)
    fs.writeFileSync(outputPath, code)

  } catch (error) {
    exitWithError(`Failed to read file: ${absolutePath}`, error)
  }
}

export default compileModule
