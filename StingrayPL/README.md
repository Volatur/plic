# StingrayPL
## Stingray is a conditionally statically typed programming language written in AtomVM for writing efficient servers on small devices like ESP32, etc
###RU:
 Stingray → Erlang BEAM (AtomVM)

## Структура / Structure 

```
src/                          # Компилятор
  stingray_tokens.erl         # Типы токенов
  stingray_lexer.erl          # Лексер (binary pattern matching)
  stingray_ast.erl            # AST конструкторы
  stingray_parser.erl         # Парсер (recursive descent)
  stingray_codegen.erl        # Кодогенератор → .beam
  stingray_runtime.erl        # Runtime-хелперы

examples/                     # Примеры на Stingray (.sr)
  hello_world.sr              # #flow:300# параллельно
  full_example.sr             # Все фичи языка
  struct_and_enum.sr          # Enum + struct
  struct_test.sr              # Struct в функциях
  enum_test.sr                # Enum сравнения
  if_else_test.sr             # if/else, логика, сравнения
  while_test.sr               # While + внешние переменные
  list_test.sr                # [], [i], .length
  string_test.sr              # Сложение строк
  push_pop_test.sr            # .push() .pop()
  all_features_test.sr        # Все фичи
  chat_server.sr              # TCP чат-сервер (на Stingray)
  chat_client.sr              # TCP чат-клиент (на Stingray)

tests/
  run_tests.erl               # 9 тестов

vscode-stingray/              # VS Code расширение
test.erl                      # Escript для компиляции .sr
stingray-lang-0.3.0.vsix      # Расширение для VS Code
```

## Быстрый старт / Fast starting 

### Компиляция модулей / Modules compilation 

```bash
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass
Remove-Item src\*.beam -Force

erlc -o src src/stingray_tokens.erl
erlc -o src src/stingray_lexer.erl
erlc -o src src/stingray_ast.erl
erlc -o src src/stingray_parser.erl
erlc -o src src/stingray_codegen.erl
erlc -o src src/stingray_runtime.erl
```

### Запуск тестов / Testing programm without compile 

```bash
erlc -o tests tests/run_tests.erl
erl -pa src -pa tests -noshell -eval "run_tests:run(), halt()."
```

### Компиляция .sr файла / .sr compilation 

```bash
escript test.erl --compile examples/hello_world.sr
```

## Синтаксис / Syntax 

### Функции / Functions

```stingray
fun main() { io.write("Hello World") }
fun add(a: Int32, b: Int32) { a + b }
```

### Переменные / Variables logic

```stingray
x: Int32 = 42
y = 3.14
name = "Stingray"
```

### Строки (сложение) / String 

```stingray
c = "Hello" + " World"         // "Hello World"
x = "Count: " + 42             // "Count: 42"
```

### if/else

```stingray
if x > 0 {
    io.write("positive")
} else {
    io.write("negative")
}

if a > 0 && b < 10 || c == 42 { io.write("ok") }
if not flag { io.write("!flag") }
```

### Циклы / Cycle

```stingray
i = 0
while i < 10 { i = i + 1 }
```

### Enum

```stingray
enum color { RED GREEN BLUE }
c = color.RED
if c == color.RED { io.write("red") }
```

### Struct

```stingray
type struct Point { Int32 x, Int32 y }
p = Point.new(10, 20)
io.write(p.x)
```

### Массивы / Array`s

```stingray
nums = [1, 2, 3]
io.write(nums[0])           // 1
io.write(nums.length)       // 3
nums2 = nums.push(4)        // [1,2,3,4]
last = nums2.pop()          // 4
```

### Параллельное выполнение / Parallel functions 

```stingray
#sideway# func()            // async
#flow:300# func()           // 300 параллельных копий/ 300 Parallel copies
```

## Чат-сервер и клиент // Server and client

### Сервер (на Stingray) / Server

```bash
# Скомпилировать
escript test.erl --compile examples/chat_server.sr

# Запустить
erl -pa src -noshell -eval "chat_server:main()."
```

### Клиент (на Stingray) / Client

```bash
# Скомпилировать
escript test.erl --compile examples/chat_client.sr

# Запустить (в другом терминале)
erl -pa src -pa examples -noshell -eval "chat_client:main()."
```

### Пример сессии // Session 

**Сервер:**
```
=== Stingray Chat Server ===
Listening on localhost:9999
Server started!
+ Client connected
msg: Hello from Stingray!
- Client disconnected
```

**Клиент:**
```
=== Stingray Chat Client ===
Connecting to localhost:9999...
Connected!
Server: echo: Hello from Stingray!
Disconnected.
```

## VS Code

```bash
code --install-extension stingray-lang-0.3.0.vsix
```

## Пайплайн

```
.sr → Lexer → Parser → AST → Codegen → Erlang forms → compile:forms → .beam
```

## Зависимости // Installation before using Stingray 

- Erlang/OTP 27+
