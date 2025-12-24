// Nu v1.5.1 语法高亮测试文件
// 测试所有关键字和语法元素

// 1. 函数定义测试
F add(a: i32, b: i32) -> i32 {
    < a + b
}

// 2. 结构体和枚举测试
S Point {
    x: f32,
    y: f32
}

E Color {
    Red,
    Green,
    Blue
}

// 3. 特性和实现测试
TR Display {
    f show() -> Str
}

I Display for Point {
    f show() -> Str {
        < "Point({}, {})"
    }
}

// 4. 主函数 - 测试所有控制流
F main() {
    // 变量声明测试
    l x = 10
    v y = 20
    a z = 30
    C MAX = 100
    
    // 打印测试
    > "x = {}", x
    > "y = {}", y
    
    // 条件判断测试
    ? x < y {
        > "x 小于 y"
    }
    
    ? x >= 10 {
        > "x 大于等于 10"
    }
    
    // Match 测试
    M x {
        10 => > "x 是 10",
        20 => > "x 是 20",
        _ => > "其他"
    }
    
    // Loop 测试
    l count = 0
    L {
        ? count >= 5 {
            b  // break
        }
        > "count = {}", count
        count = count + 1
        
        ? count == 3 {
            c  // continue
        }
    }
    
    // While 循环测试
    v i = 0
    wh i < 5 {
        > "i = {}", i
        i = i + 1
    }
    
    // 类型测试
    l s: Str = "Hello"
    l vec: V<i32> = V::new()
    l opt: O<i32> = O::Some(42)
    l res: R<i32, Str> = R::Ok(100)
    
    // 异步测试
    ~ async_function()
    
    // 并发测试
    @ spawn_task()
    @@ thread_task()
    
    // Try 操作符测试
    l result = risky_operation()!
    
    // 比较操作符测试
    l is_equal = x == y
    l is_not_equal = x != y
    l is_less_equal = x <= y
    l is_greater_equal = x >= y
    
    < 0
}

// 5. 属性测试
#D
S DerivedStruct {
    field: i32
}

#I
f inline_function() {
    < 42
}

#?
f test_function() {
    l result = add(1, 2)
    ? result == 3 {
        > "Test passed!"
    }
}

// 6. 外部函数测试
EXT f external_function(x: i32) -> i32

// 7. 静态变量测试
ST l GLOBAL_VAR: i32 = 100

// 8. 委托测试
D MyDelegate(i32) -> i32

// 9. 原始类型测试
f type_test() {
    l a: u8 = 255
    l b: u16 = 65535
    l c: u32 = 4294967295
    l d: u64 = 18446744073709551615
    l e: i8 = -128
    l f: i16 = -32768
    l g: i32 = -2147483648
    l h: i64 = -9223372036854775808
    l i: f32 = 3.14
    l j: f64 = 2.718281828
    l k: bool = true
    l m: char = 'A'
    l n: usize = 1000
    l o: isize = -1000
}

// 10. 字符串和转义测试
f string_test() {
    l s1 = "Hello, World!"
    l s2 = "Line 1\nLine 2"
    l s3 = "Tab\there"
    l s4 = "Quote: \"test\""
    < s1
}

// 11. 数字常量测试
f number_test() {
    l int_val = 42
    l float_val = 3.14159
    l bool_true = true
    l bool_false = false
}

// 12. 块注释测试
/*
这是一个多行块注释
可以跨越多行
用于详细说明
*/

/* 单行块注释 */

// 13. 复杂表达式测试
f complex_expression() {
    l result = (x + y) * z - w / v
    l comparison = (a <= b) && (c >= d) || (e == f) && (g != h)
    < result
}