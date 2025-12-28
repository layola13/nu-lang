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

struct Operator {
    std::string op;
};

struct Operator {
private:
    static Operator from_str(std::string s) {
        // TODO: parse body
    }
};

