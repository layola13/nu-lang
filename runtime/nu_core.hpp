// Nu Runtime Core Library for C++23
// nu_core.hpp - 提供 Nu-lang 到 C++23 的运行时支持
//
// 编译要求: GCC 14+ / Clang 17+ with -std=c++23

#ifndef NU_CORE_HPP
#define NU_CORE_HPP

#include <cstdint>
#include <string>
#include <string_view>
#include <vector>
#include <memory>
#include <optional>
#include <expected>
#include <variant>
#include <mutex>
#include <thread>
#include <print>
#include <format>
#include <stdexcept>
#include <utility>

namespace nu {

// ============================================================================
// Error Types
// ============================================================================

/// 通用错误类型 (用于 std::expected<T, nu::Error>)
struct Error {
    std::string message;
    
    explicit Error(std::string msg) : message(std::move(msg)) {}
    explicit Error(const char* msg) : message(msg) {}
    
    auto what() const -> std::string_view { return message; }
};

// ============================================================================
// NU_TRY 宏 - 模拟 Rust 的 ? 运算符
// ============================================================================

/// 用于展开 std::expected，如果失败则返回错误
/// 用法: auto val = NU_TRY(some_function_returning_expected());
#define NU_TRY(expr) \
    ({ \
        auto&& _result = (expr); \
        if (!_result.has_value()) { \
            return std::unexpected(_result.error()); \
        } \
        std::move(_result.value()); \
    })

/// 用于展开 std::expected，如果失败则 panic (unwrap)
/// 用法: auto val = NU_UNWRAP(some_function_returning_expected());
#define NU_UNWRAP(expr) \
    ({ \
        auto&& _result = (expr); \
        if (!_result.has_value()) { \
            throw std::runtime_error(_result.error().message); \
        } \
        std::move(_result.value()); \
    })

// ============================================================================
// Mutex<T> - 模拟 Rust 的 Mutex (数据和锁绑定在一起)
// ============================================================================

/// Rust 风格的 Mutex - 保护数据而非代码块
/// 使用方法:
///   auto m = nu::Mutex<int32_t>(5);
///   {
///       auto guard = NU_TRY(m.lock());
///       *guard = 10;
///   } // 自动解锁
template <typename T>
class Mutex {
    mutable std::mutex _mtx;
    T _data;

public:
    /// 完美转发构造
    template<typename... Args>
    explicit Mutex(Args&&... args) : _data(std::forward<Args>(args)...) {}
    
    // 禁止拷贝
    Mutex(const Mutex&) = delete;
    Mutex& operator=(const Mutex&) = delete;
    
    // 允许移动 (需要锁定)
    Mutex(Mutex&& other) noexcept {
        std::lock_guard lock(other._mtx);
        _data = std::move(other._data);
    }
    
    Mutex& operator=(Mutex&& other) noexcept {
        if (this != &other) {
            std::scoped_lock lock(_mtx, other._mtx);
            _data = std::move(other._data);
        }
        return *this;
    }

    /// RAII LockGuard - 模拟 Rust 的 MutexGuard<T>
    class LockGuard {
        std::unique_lock<std::mutex> _lock;
        T& _ref;
        
    public:
        LockGuard(std::mutex& m, T& d) : _lock(m), _ref(d) {}
        
        // 像指针一样访问数据
        auto operator->() -> T* { return &_ref; }
        auto operator->() const -> const T* { return &_ref; }
        auto operator*() -> T& { return _ref; }
        auto operator*() const -> const T& { return _ref; }
        
        // 获取原始引用
        auto get() -> T& { return _ref; }
        auto get() const -> const T& { return _ref; }
    };

    /// 获取锁 - 返回 expected<LockGuard, Error>
    /// Rust: mutex.lock().unwrap() -> NU_TRY(mutex.lock()) 或 NU_UNWRAP(mutex.lock())
    auto lock() -> std::expected<LockGuard, Error> {
        try {
            return LockGuard(_mtx, _data);
        } catch (const std::exception& e) {
            return std::unexpected(Error(e.what()));
        }
    }
    
    /// 尝试获取锁 (非阻塞)
    auto try_lock() -> std::optional<LockGuard> {
        if (_mtx.try_lock()) {
            // 手动构造 unique_lock (已经锁定)
            return LockGuard(_mtx, _data);
        }
        return std::nullopt;
    }
};

// ============================================================================
// RwLock<T> - 模拟 Rust 的 RwLock (读写锁)
// ============================================================================

template <typename T>
class RwLock {
    mutable std::shared_mutex _mtx;
    T _data;

public:
    template<typename... Args>
    explicit RwLock(Args&&... args) : _data(std::forward<Args>(args)...) {}
    
    RwLock(const RwLock&) = delete;
    RwLock& operator=(const RwLock&) = delete;

    /// 读取 Guard
    class ReadGuard {
        std::shared_lock<std::shared_mutex> _lock;
        const T& _ref;
    public:
        ReadGuard(std::shared_mutex& m, const T& d) : _lock(m), _ref(d) {}
        auto operator->() const -> const T* { return &_ref; }
        auto operator*() const -> const T& { return _ref; }
    };
    
    /// 写入 Guard
    class WriteGuard {
        std::unique_lock<std::shared_mutex> _lock;
        T& _ref;
    public:
        WriteGuard(std::shared_mutex& m, T& d) : _lock(m), _ref(d) {}
        auto operator->() -> T* { return &_ref; }
        auto operator*() -> T& { return _ref; }
    };
    
    auto read() -> std::expected<ReadGuard, Error> {
        return ReadGuard(_mtx, _data);
    }
    
    auto write() -> std::expected<WriteGuard, Error> {
        return WriteGuard(_mtx, _data);
    }
};

// ============================================================================
// Utility Functions
// ============================================================================

/// 类似 Rust 的 panic!
[[noreturn]] inline void panic(std::string_view msg) {
    std::println(stderr, "PANIC: {}", msg);
    std::terminate();
}

/// 类似 Rust 的 unreachable!
[[noreturn]] inline void unreachable() {
    panic("entered unreachable code");
}

/// 类似 Rust 的 todo!
[[noreturn]] inline void todo() {
    panic("not yet implemented");
}

/// unimplemented! (同 todo)
[[noreturn]] inline void unimplemented() {
    todo();
}

// ============================================================================
// Hash Combine (for derive(Hash) support)
// ============================================================================

/// Combine hash values (Boost-style)
/// Usage in std::hash<T> specialization:
///   std::size_t seed = 0;
///   nu::hash_combine(seed, std::hash<int>{}(obj.a));
///   nu::hash_combine(seed, std::hash<std::string>{}(obj.b));
///   return seed;
inline void hash_combine(std::size_t& seed, std::size_t value) {
    seed ^= value + 0x9e3779b9 + (seed << 6) + (seed >> 2);
}

// ============================================================================
// Type Aliases (for convenience)
// ============================================================================

/// Box<T> alias
template<typename T>
using Box = std::unique_ptr<T>;

/// Arc<T> alias (C++ shared_ptr 默认是原子的)
template<typename T>
using Arc = std::shared_ptr<T>;

/// Rc<T> alias (降级为 shared_ptr)
template<typename T>
using Rc = std::shared_ptr<T>;

/// Weak<T> alias
template<typename T>
using Weak = std::weak_ptr<T>;

/// Option<T> alias
template<typename T>
using Option = std::optional<T>;

/// Result<T, E> alias
template<typename T, typename E = Error>
using Result = std::expected<T, E>;

// ============================================================================
// Smart Pointer Constructors (模拟 Rust 的 ::new)
// ============================================================================

/// Box::new(val) -> make_box(val)
template<typename T, typename... Args>
auto make_box(Args&&... args) -> Box<T> {
    return std::make_unique<T>(std::forward<Args>(args)...);
}

/// Arc::new(val) -> make_arc(val)
template<typename T, typename... Args>
auto make_arc(Args&&... args) -> Arc<T> {
    return std::make_shared<T>(std::forward<Args>(args)...);
}

/// Rc::new(val) -> make_rc(val) (same as Arc in C++)
template<typename T, typename... Args>
auto make_rc(Args&&... args) -> Rc<T> {
    return std::make_shared<T>(std::forward<Args>(args)...);
}

} // namespace nu

#endif // NU_CORE_HPP
