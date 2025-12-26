// 测试 SourceMap 生成 - Nu v1.6.3 标准
F add(a: i32, b: i32) -> i32 {
    a + b
}

F multiply(x: i32, y: i32) -> i32 {
    x * y
}

f main() {
    l result = add(10, 20);
    println!("Result: {}", result);
    
    l product = multiply(5, 6);
    println!("Product: {}", product);
}