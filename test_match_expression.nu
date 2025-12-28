
E Shape {
    Circle(f64),
    Rectangle(f64, f64),
    Triangle(f64, f64, f64),
}

F calculate_area(shape: Shape) -> f64 {
    M shape {
        Circle(r) => 3.14159 * r * r,
        Rectangle(w, h) => w * h,
        Triangle(a, b, c) => {
            l s = (a + b + c) / 2.0
            < (s * (s - a) * (s - b) * (s - c)).sqrt()
        },
        _ => 0.0,
    }
}
