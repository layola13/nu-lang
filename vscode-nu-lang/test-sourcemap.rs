// 测试 SourceMap 生成 - Nu v1.6.3 标准
pub fn add(a: i32, b: i32) -> i32 {
a + b
}

pub fn multiply(x: i32, y: i32) -> i32 {
x * y
}

fn main() {
let result = add(10, 20);
println!("Result: {}", result);

let product = multiply(5, 6);
println!("Product: {}", product);
}
