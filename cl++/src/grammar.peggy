Program
    = _ func:FunctionDeclaration _ {
        return func
    }


FunctionDeclaration
    = "fn " name:Identifier "()" _ "{" _ body:[^}]+ _ "}" {
        return {
            type: "FunctionDeclaration",
            name: name,
            body: body.join("").trim()
        }
    }

Identifier
    = [a-zA-Z_][a-zA-Z0-9_]* {return text()}

_ "whitespace"
    = [ \t\n\r]*
