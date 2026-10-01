/*
Добавить зависимость в [build-dependencies]
    cargo add cc --build

*/
// данная структура должна иметь такое же расположение в памяти (layout), 
// как если бы она была объявлена на языке C
#[repr(C)]
struct Point {
    x: f64,
    y: f64,
}

// блок, который объявляет функции, реализованные вне текущего крейта 
// (обычно на языке C или в любой библиотеке, использующей C ABI 
//   [Application Binary Interface]).
// функция создана в файле native\distance.c
unsafe extern "C" {
    fn distance(p1: Point, p2: Point) ->f64;
}

fn main() {
    let p1 = Point {x: 0.0, y: 0.0} ;
    let p2 = Point {x: 10.0, y: 10.0} ;

    println!(
        "Distance: {}",
        unsafe{
            distance(p1, p2)
        }
    ) ; // Out: Distance: 14.142135623730951
}
