# FFI с C в Rust

У вас получился **канонический пример** интеграции C-кода в Rust через FFI (Foreign Function Interface). Разберём всё по порядку: от структуры проекта до тонкостей ABI.

---

## Структура проекта

Добавить зависимость в раздел `[build-dependencies]` файла `Cargo.toml`:
```bash
cargo add cc --build
```

```
bookstore-service/
├── Cargo.toml
├── build.rs                    ← скрипт сборки
├── src/
│   └── main.rs                 ← Rust-код с FFI-объявлениями
└── native/
    └── distance.c              ← C-код
```

**`Cargo.toml`:**

```toml
[package]
name = "bookstore-service"
version = "0.1.0"
edition = "2024"

[build-dependencies]
cc = "1"
```

---

## Как это работает по шагам

### 1. Cargo видит `build.rs` в корне пакета

Cargo **автоматически** обнаруживает файл `build.rs` в корне пакета и выполняет его **до** компиляции крейта. Это называется **build script**.

Порядок:
```
build.rs → компиляция Rust-крейта → линковка → бинарник
```

### 2. `build.rs` вызывает `cc::Build`

```rust
fn main() {
    cc::Build::new()
        .file("native/distance.c")
        .compile("distance");
}
```

Что происходит:

| Шаг | Что делает |
|---|---|
| `cc::Build::new()` | создаёт билдер для C-компилятора |
| `.file("native/distance.c")` | добавляет исходник |
| `.compile("distance")` | компилирует в статическую библиотеку `libdistance.a` (Linux/macOS) или `distance.lib` (Windows) |

`cc` автоматически:
- находит системный C-компилятор (`gcc`, `clang`, `cl.exe`);
- применяет нужные флаги под целевую платформу;
- кладёт результат в `target/<profile>/build/.../out/`;
- **сообщает Cargo** через `cargo:rustc-link-lib` и `cargo:rustc-link-search`, что нужно слинковать `libdistance`.

### 3. Rust-код объявляет внешнюю функцию

```rust
unsafe extern "C" {
    fn distance(p1: Point, p2: Point) -> f64;
}
```

- `extern "C"` — функция использует **C ABI** (соглашение о вызовах: порядок аргументов, кто чистит стек, как передаются структуры).
- `unsafe` (в edition 2024) — объявление внешних функций требует `unsafe`, потому что Rust не может проверить их корректность.
- **Тело отсутствует** — это лишь декларация, линкер найдёт реализацию в `libdistance`.

### 4. Линковка

При сборке финального бинарника линкер соединяет:
- скомпилированный Rust-код;
- `libdistance.a` из `build.rs`.

Без `build.rs` линкер бы выдал:
```
undefined reference to `distance'
```

---

## Ключевой момент — `#[repr(C)]`

```rust
#[repr(C)]
struct Point {
    x: f64,
    y: f64,
}
```

### Почему это критично

По умолчанию Rust **не гарантирует** порядок полей в структуре — компилятор может их переставить для оптимизации (экономия памяти, выравнивание). Это называется **repr(Rust)**.

Если передать такую структуру в C, C-код прочитает поля **не там, где ожидает**, и получит мусор.

`#[repr(C)]` заставляет Rust раскладывать поля:
- **в порядке объявления**;
- с **выравниванием по правилам C ABI**;
- с теми же padding-байтами, что и C-компилятор.

### Соответствие типов

| Rust | C |
|---|---|
| `f64` | `double` |
| `f32` | `float` |
| `i32` | `int32_t` / `int` |
| `u8` | `uint8_t` / `unsigned char` |
| `*const T` | `const T*` |
| `*mut T` | `T*` |
| `()` | `void` |
| `#[repr(C)] struct` | `struct` |

Ваш `Point { x: f64, y: f64 }` ↔ `typedef struct { double x; double y; } Point` — идеальное совпадение.

---

## FFI-safe типы

Не всё можно передавать через FFI. Правило: тип должен иметь **определённый layout**, одинаковый в обоих языках.

### ✅ Можно

- примитивы: `i8..i64`, `u8..u64`, `f32`, `f64`, `bool` (как `_Bool`), `char` (как `u32`);
- сырые указатели: `*const T`, `*mut T`;
- `#[repr(C)]` структуры;
- `#[repr(C)]` / `#[repr(u32)]` enum;
- `Option<&T>` и `Option<extern "C" fn>` (нишевая оптимизация для nullable-указателей);
- `#[repr(transparent)]` обёртки.

### ❌ Нельзя напрямую

| Тип | Почему | Решение |
|---|---|---|
| `String` | нет гарантированного layout | `*const c_char` + длина |
| `&str` | не NUL-terminated, нет layout | `*const c_char` |
| `Vec<T>` | внутренний layout не определён | `*mut T` + len + capacity |
| `Box<T>` | layout не определён | `*mut T` |
| trait-объекты | vtable не совместим с C | функции-обёртки |
| `HashMap`, `BTreeMap` | внутренняя структура | не передавать |

### Пример: передача строки в C

```rust
use std::ffi::{CString, CStr};
use std::os::raw::c_char;

unsafe extern "C" {
    fn strlen(s: *const c_char) -> usize;
}

fn main() {
    let s = CString::new("hello").unwrap(); // добавляет \0
    let len = unsafe { strlen(s.as_ptr()) };
    println!("{len}"); // 5
}
```

---

## `unsafe extern "C"` — что изменилось в edition 2024

До edition 2024:

```rust
extern "C" {
    fn distance(p1: Point, p2: Point) -> f64;
}
```

В edition 2024 **объявление** внешних функций требует `unsafe`:

```rust
unsafe extern "C" {
    fn distance(p1: Point, p2: Point) -> f64;
}
```

Смысл: подчеркнуть, что **само существование** такой функции — контракт, который Rust не может проверить. Вызов всё равно требует `unsafe { ... }`:

```rust
unsafe { distance(p1, p2) }
```

---

## Разбор C-кода

```c
#include <math.h>

typedef struct {
    double x;
    double y;
} Point;

double distance(Point p1, Point p2) {
    double dx = p2.x - p1.x;
    double dy = p2.y - p1.y;
    return sqrt(dx * dx + dy * dy);
}
```

Здесь:
- `#include <math.h>` — для `sqrt`;
- `typedef struct { double x; double y; } Point;` — анонимная структура + псевдоним;
- `double distance(Point p1, Point p2)` — функция принимает **две структуры по значению**.

⚠️ Передача структур **по значению** через FFI — тонкий момент. Разные платформы по-разному передают большие структуры (в регистрах или через стек). `#[repr(C)]` + C ABI гарантируют совместимость, но для сложных структур надёжнее передавать **указатели**.

### Альтернатива через указатели

```c
double distance(const Point* p1, const Point* p2) {
    double dx = p2->x - p1->x;
    double dy = p2->y - p1->y;
    return sqrt(dx * dx + dy * dy);
}
```

```rust
unsafe extern "C" {
    fn distance(p1: *const Point, p2: *const Point) -> f64;
}

let d = unsafe { distance(&p1, &p2) };
```

Это **более переносимо** и рекомендовано для FFI.

---

## Полная цепочка сборки

```
1. cargo build
        │
        ▼
2. Cargo находит build.rs
        │
        ▼
3. build.rs запускается как отдельный Rust-бинарник
        │
        ▼
4. cc::Build компилирует native/distance.c
   → target/debug/build/<pkg>/out/libdistance.a
        │
        ▼
5. build.rs печатает в stdout:
   cargo:rustc-link-search=...
   cargo:rustc-link-lib=static=distance
        │
        ▼
6. Cargo компилирует src/main.rs
        │
        ▼
7. Линкер соединяет Rust-объект + libdistance.a
        │
        ▼
8. Бинарник готов
```

---

## Что нужно знать про `cc` crate

```toml
[build-dependencies]
cc = "1"
```

`cc` — стандартный способ компилировать C/C++ из `build.rs`. Основные методы:

| Метод | Назначение |
|---|---|
| `.file("x.c")` | добавить исходник |
| `.files(["a.c", "b.c"])` | несколько |
| `.include("dir")` | `-I` путь |
| `.flag("-O3")` | флаг компилятора |
| `.define("FOO", "1")` | `-DFOO=1` |
| `.warnings(true)` | включить предупреждения |
| `.opt_level(3)` | уровень оптимизации |
| `.compile("name")` | собрать в `lib<name>.a` |
| `.cargo_metadata(true)` | (по умолчанию) сообщить Cargo о линковке |

Для C++ — то же самое, но `cc` определяет язык по расширению (`.cpp`, `.cc`) или через `.cpp(true)`.

---

## Проверка, что всё слинковалось

```bash
cargo build -vv 2>&1 | grep -i distance
```

Вы увидите что-то вроде:

```
Running `cc ... -o libdistance.a`
cargo:rustc-link-lib=static=distance
cargo:rustc-link-search=native=...
```

---

## Типичные ошибки

### `undefined reference to 'distance'`

Причины:
- нет `build.rs`;
- `cc` не вызывается;
- неверное имя библиотеки в `.compile("distance")` vs `-l distance`;
- бинарник не пересобрался — попробуйте `cargo clean`.

### `invalid repr` / мусор в полях

Забыли `#[repr(C)]` на структуре.

### `error: extern blocks must be unsafe` (edition 2024)

Добавьте `unsafe` перед `extern "C"`.

### `linking with 'cc' failed`

Нет C-компилятора в системе. Установите `gcc`/`clang`/MSVC Build Tools.

### Несовпадение размеров `Point`

Проверьте, что в C и Rust **одинаковые типы полей** и порядок. `#[repr(C)]` не спасёт, если в C `float`, а в Rust `f64`.

---

## Расширение: несколько функций и заголовок

Если функций много, удобно сделать `native/distance.h`:

```c
#ifndef DISTANCE_H
#define DISTANCE_H

typedef struct {
    double x;
    double y;
} Point;

double distance(Point p1, Point p2);

#endif
```

И в Rust — модуль `ffi.rs`:

```rust
#[repr(C)]
pub struct Point { pub x: f64, pub y: f64 }

unsafe extern "C" {
    pub fn distance(p1: Point, p2: Point) -> f64;
}
```

Либо использовать [`bindgen`](https://github.com/rust-lang/rust-bindgen) для автогенерации Rust-биндингов из `.h`:

```toml
[build-dependencies]
bindgen = "0.70"
```

---

## Итоговая таблица

| Элемент | Роль |
|---|---|
| `build.rs` | скрипт, запускаемый до компиляции крейта |
| `[build-dependencies] cc` | компилятор C из build-скрипта |
| `cc::Build::new().file(...).compile("name")` | собрать C в `libname.a` и сообщить Cargo |
| `unsafe extern "C" { ... }` | объявление внешних функций с C ABI |
| `#[repr(C)]` | гарантировать layout, совместимый с C |
| `unsafe { distance(...) }` | вызов внешней функции |
| FFI-safe типы | примитивы, указатели, `#[repr(C)]` структуры |
| `CString`/`CStr` | безопасная работа со строками C |

**Суть:** `build.rs` + `cc` компилируют C-код в статическую библиотеку; `#[repr(C)]` и `extern "C"` обеспечивают совместимость ABI; линкер соединяет всё в один бинарник. Ваш пример — минимальный, но полностью корректный шаблон FFI-интеграции.
