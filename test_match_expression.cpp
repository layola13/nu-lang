#include <cstdint>
#include <string>
#include <iostream>
#include <vector>
#include <memory>
#include <optional>


enum Shape {
    Circle(double),
    Rectangle(double, double),
    Triangle(double, double, double),
}

double calculate_area(Shape shape) {
    switch (shape) {
        Circle(r) => 3.14159 * r * r,
        Rectangle(w, h) => w * h,
        Triangle(a, b, c) => {
            const auto s = (a + b + c) / 2.0
            return (s * (s - a) * (s - b) * (s - c)).sqrt();
        },
        _ => 0.0,
    }
}
