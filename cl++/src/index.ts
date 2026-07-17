import compileModule from "./compiler.js"
import exitWithError from "./utils/exitWithError.js"

const main = () => {
  const filePath = process.argv[2]
  if (!filePath) exitWithError("Error: path to entry .clx file is required")

  compileModule(filePath)
}

main()
