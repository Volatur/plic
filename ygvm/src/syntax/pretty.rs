//! Pretty printer для AST – вывод в стиле Lisp/Clojure.

use super::parser::*;

/// Форматирует AST-узел в строку с отступами.
pub fn pretty_print(module: &Module) -> String {
    format_module(module, 0)
}

// ---- Модуль ----

fn format_module(module: &Module, indent: usize) -> String {
    let mut s = String::new();
    s.push_str(&format!("{}(module {}\n", " ".repeat(indent), module.path));
    for item in &module.items {
        s.push_str(&format_item(item, indent + 2));
    }
    s.push_str(&format!("{})\n", " ".repeat(indent)));
    s
}

fn format_item(item: &Item, indent: usize) -> String {
    match item {
        Item::Function(f) => format_function(f, indent),
        Item::Class(c) => format_class(c, indent, "class"),
        Item::Object(o) => format_class(o, indent, "object"),
    }
}

// ---- Функции ----

fn format_function(func: &Function, indent: usize) -> String {
    let mut s = String::new();
    let name = if func.name.is_empty() { "anon" } else { &func.name };
    s.push_str(&format!("{}(fun {}\n", " ".repeat(indent), name));

    let params_str = if func.params.is_empty() {
        "()".to_string()
    } else {
        format!("({})", func.params.join(" "))
    };
    s.push_str(&format!("{}  {}\n", " ".repeat(indent), params_str));

    match &func.captures {
        None => {
            s.push_str(&format!("{}  (captures *)\n", " ".repeat(indent)));
        }
        Some(caps) if !caps.is_empty() => {
            let caps_str = caps.join(" ");
            s.push_str(&format!("{}  (captures {})\n", " ".repeat(indent), caps_str));
        }
        Some(_) => {}
    }

    s.push_str(&format_block(&func.body, indent + 2));
    s.push_str(&format!("{})\n", " ".repeat(indent)));
    s
}

// ---- Классы и объекты ----

fn format_class(cls: &Class, indent: usize, kind: &str) -> String {
    let mut s = String::new();
    s.push_str(&format!("{}({} {}\n", " ".repeat(indent), kind, cls.name));

    if !cls.extends.is_empty() {
        let extends_str = cls.extends.join(" ");
        s.push_str(&format!("{}  (extends {})\n", " ".repeat(indent), extends_str));
    }

    for method in &cls.methods {
        s.push_str(&format_method(method, indent + 2));
    }
    s.push_str(&format!("{})\n", " ".repeat(indent)));
    s
}

fn format_method(method: &Method, indent: usize) -> String {
    let mut s = String::new();
    let params_str = if method.params.is_empty() {
        "()".to_string()
    } else {
        format!("({})", method.params.join(" "))
    };
    s.push_str(&format!("{}(def {} {}\n", " ".repeat(indent), method.name, params_str));
    s.push_str(&format_block(&method.body, indent + 2));
    s.push_str(&format!("{})\n", " ".repeat(indent)));
    s
}

// ---- Блоки и операторы ----

fn format_block(block: &Block, indent: usize) -> String {
    let mut s = String::new();
    for stmt in &block.statements {
        s.push_str(&format_statement(stmt, indent));
    }
    s
}

fn format_statement(stmt: &Statement, indent: usize) -> String {
    match stmt {
        Statement::Let(let_stmt) => {
            format!("{}(let {} {})\n", " ".repeat(indent), let_stmt.name, format_expr(&let_stmt.value))
        }
        Statement::Assign(assign) => {
            let target = match &assign.target {
                AssignTarget::Var(name) => name.clone(),
                AssignTarget::Index { target, index } => format!("(get {} {})", format_expr(target), format_expr(index)),
                AssignTarget::Member { target, name } => format!("(. {} {})", format_expr(target), name),
            };
            let op = match assign.op {
                AssignOp::Assign => "=",
                AssignOp::PlusEq => "=+",
                AssignOp::MinusEq => "=-",
                AssignOp::StarEq => "=*",
                AssignOp::SlashEq => "=/",
                AssignOp::PercentEq => "=%",
                AssignOp::AndEq => "=&",
                AssignOp::OrEq => "=|",
                AssignOp::XorEq => "=^",
                AssignOp::NotEq => "=!",
                AssignOp::LtEq => "=<",
                AssignOp::GtEq => "=>",
                AssignOp::LeEq => "=<=",
                AssignOp::GeEq => "=>=",
            };
            format!("{}(set! {} {} {})\n", " ".repeat(indent), target, op, format_expr(&assign.value))
        }
        Statement::If(if_stmt) => {
            let mut s = String::new();
            s.push_str(&format!("{}(if {}\n", " ".repeat(indent), format_expr(&if_stmt.cond)));
            s.push_str(&format_block(&if_stmt.then_block, indent + 2));
            if let Some(else_block) = &if_stmt.else_block {
                s.push_str(&format!("{}  (else\n", " ".repeat(indent)));
                s.push_str(&format_block(else_block, indent + 4));
                s.push_str(&format!("{}  )\n", " ".repeat(indent)));
            }
            s.push_str(&format!("{})\n", " ".repeat(indent)));
            s
        }
        Statement::While(while_stmt) => {
            let mut s = String::new();
            s.push_str(&format!("{}(while {}\n", " ".repeat(indent), format_expr(&while_stmt.cond)));
            s.push_str(&format_block(&while_stmt.body, indent + 2));
            s.push_str(&format!("{})\n", " ".repeat(indent)));
            s
        }
        Statement::For(for_stmt) => {
            let mut s = String::new();
            s.push_str(&format!("{}(for {} (in {})\n", " ".repeat(indent), for_stmt.var, format_expr(&for_stmt.iter)));
            s.push_str(&format_block(&for_stmt.body, indent + 2));
            s.push_str(&format!("{})\n", " ".repeat(indent)));
            s
        }
        Statement::Return(expr) => {
            if let Some(e) = expr {
                format!("{}(return {})\n", " ".repeat(indent), format_expr(e))
            } else {
                format!("{}(return)\n", " ".repeat(indent))
            }
        }
        Statement::TryCatch(tc) => {
            let mut s = String::new();
            s.push_str(&format!("{}(try\n", " ".repeat(indent)));
            s.push_str(&format_block(&tc.try_block, indent + 2));
            s.push_str(&format!("{}  (catch ({})\n", " ".repeat(indent), tc.catch_param));
            s.push_str(&format_block(&tc.catch_block, indent + 4));
            s.push_str(&format!("{}  )\n", " ".repeat(indent)));
            s.push_str(&format!("{})\n", " ".repeat(indent)));
            s
        }
        Statement::Throw(expr) => {
            format!("{}(throw {})\n", " ".repeat(indent), format_expr(expr))
        }
        Statement::Expr(expr) => {
            format!("{}{}\n", " ".repeat(indent), format_expr(expr))
        }
        Statement::Block(block) => {
            let mut s = String::new();
            s.push_str(&format!("{}(do\n", " ".repeat(indent)));
            s.push_str(&format_block(block, indent + 2));
            s.push_str(&format!("{})\n", " ".repeat(indent)));
            s
        }
    }
}

// ---- Выражения ----

fn format_expr(expr: &Expr) -> String {
    match expr {
        Expr::Literal(lit) => format_literal(lit),
        Expr::Var(name) => name.clone(),
        Expr::Binary(bin) => {
            format!("({} {} {})",
                    match bin.op {
                        BinaryOp::Add => "+",
                        BinaryOp::Sub => "-",
                        BinaryOp::Mul => "*",
                        BinaryOp::Div => "/",
                        BinaryOp::Mod => "%",
                        BinaryOp::And => "and",
                        BinaryOp::Or => "or",
                        BinaryOp::Xor => "xor",
                        BinaryOp::Eq => "==",
                        BinaryOp::Neq => "!=",
                        BinaryOp::Lt => "<",
                        BinaryOp::Gt => ">",
                        BinaryOp::Le => "<=",
                        BinaryOp::Ge => ">=",
                    },
                    format_expr(&bin.left),
                    format_expr(&bin.right)
            )
        }
        Expr::Unary(unary) => {
            format!("({} {})",
                    match unary.op {
                        UnaryOp::Neg => "-",
                        UnaryOp::Not => "not",
                    },
                    format_expr(&unary.expr)
            )
        }
        Expr::Call(call) => {
            let args: Vec<String> = call.args.iter().map(|a| format_expr(a)).collect();
            format!("({} {})", format_expr(&call.callee), args.join(" "))
        }
        Expr::Index(index) => {
            format!("(get {} {})", format_expr(&index.target), format_expr(&index.index))
        }
        Expr::Member(mem) => {
            format!("(. {} {})", format_expr(&mem.target), mem.name)
        }
        Expr::MethodCallColon(mc) => {
            let args: Vec<String> = mc.args.iter().map(|a| format_expr(a)).collect();
            format!("({}:{} {})", mc.class, mc.method, args.join(" "))
        }
        Expr::MethodCallDot(mc) => {
            let args: Vec<String> = mc.args.iter().map(|a| format_expr(a)).collect();
            if args.is_empty() {
                format!("(. {} {})", format_expr(&mc.target), mc.method)
            } else {
                format!("(. {} {} {})", format_expr(&mc.target), mc.method, args.join(" "))
            }
        }
        Expr::ModuleCall(mc) => {
            let args: Vec<String> = mc.args.iter().map(|a| format_expr(a)).collect();
            format!("({}::{} {})", mc.module, mc.function, args.join(" "))
        }
        Expr::New(new_expr) => {
            let args: Vec<String> = new_expr.args.iter().map(|a| format_expr(a)).collect();
            format!("(new {} {})", new_expr.class, args.join(" "))
        }
        Expr::Function(func) => {
            let params_str = if func.params.is_empty() {
                "()".to_string()
            } else {
                format!("({})", func.params.join(" "))
            };
            let body_str = if func.body.statements.is_empty() {
                "(do)".to_string()
            } else {
                let mut body = String::new();
                for stmt in &func.body.statements {
                    body.push_str(&format_statement(stmt, 0));
                }
                body
            };
            format!("(fn {} {})", params_str, body_str.trim())
        }
        Expr::ObjectRef(path) => format!("(object-ref {})", path),
    }
}

fn format_literal(lit: &Literal) -> String {
    match lit {
        Literal::Integer(n) => n.to_string(),
        Literal::Double(f) => f.to_string(),
        Literal::String(s) => format!("\"{}\"", s),
        Literal::Boolean(b) => b.to_string(),
        Literal::Array(elems) => {
            let items: Vec<String> = elems.iter().map(|e| format_expr(e)).collect();
            format!("[{}]", items.join(" "))
        }
        Literal::Map(pairs) => {
            let items: Vec<String> = pairs.iter()
                .map(|(k, v)| format!("{} {}", format_expr(k), format_expr(v)))
                .collect();
            format!("{{{}}}", items.join(" "))
        }
    }
}