#include <cstdint>
#include <string>
#include <string_view>
#include <vector>
#include <memory>
#include <optional>
#include <expected>
#include <print>
#include <format>
#include <variant>
#include <thread>

enum class Operator {
    Add,
    Subtract,
    Multiply,
    Divide
};

struct DivisionByZero {
};

struct InvalidOperator {
    std::string _0;
};

struct ParseError {
    std::string _0;
};

using CalcError = std::variant<DivisionByZero, InvalidOperator, ParseError>;

// Implementation for Operator with methods:
  std::expected<Operator, CalcError> from_str(...)

