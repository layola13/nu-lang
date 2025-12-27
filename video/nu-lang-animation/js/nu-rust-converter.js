/**
 * Nu → Rust 实时转换器
 * 简化版本，用于演示目的
 */

const NuRustConverter = {
    /**
     * 将 Nu 代码转换为 Rust 代码
     */
    convert(nuCode) {
        let rustCode = nuCode;

        // 关键字转换映射
        const conversions = [
            // 函数定义
            { pattern: /^F\s+/gm, replacement: 'pub fn ' },
            { pattern: /^f\s+/gm, replacement: 'fn ' },
            
            // 变量定义
            { pattern: /\bl\s+/g, replacement: 'let ' },
            { pattern: /\bv\s+/g, replacement: 'let mut ' },
            
            // 结构体和枚举
            { pattern: /^S\s+/gm, replacement: 'pub struct ' },
            { pattern: /^E\s+/gm, replacement: 'pub enum ' },
            
            // Impl
            { pattern: /^I\s+/gm, replacement: 'impl ' },
            
            // 控制流
            { pattern: /^\s*<\s+/gm, replacement: '    return ' },
            { pattern: /\?\s+/g, replacement: 'if ' },
            { pattern: /\bM\s+/g, replacement: 'match ' },
            { pattern: /\bL\s+/g, replacement: 'for ' },
            
            // 其他关键字
            { pattern: /\bu\s+/g, replacement: 'use ' },
            { pattern: /\bt\s+/g, replacement: 'type ' },
            { pattern: /\bwh\s+/g, replacement: 'where ' },
            { pattern: /\ba\s+/g, replacement: 'as ' },
            { pattern: /\bbr\b/g, replacement: 'break' },
            { pattern: /\bct\b/g, replacement: 'continue' },
            
            // 类型缩写
            { pattern: /\bV</g, replacement: 'Vec<' },
            { pattern: /\bO</g, replacement: 'Option<' },
            { pattern: /\bR</g, replacement: 'Result<' },
            { pattern: /\bA</g, replacement: 'Arc<' },
            { pattern: /\bX</g, replacement: 'Mutex<' },
            { pattern: /\bB</g, replacement: 'Box<' },
            
            // 模块
            { pattern: /^D\s+/gm, replacement: 'mod ' },
            
            // 常量
            { pattern: /^C\s+/gm, replacement: 'const ' },
            { pattern: /^ST\s+/gm, replacement: 'static ' },
            { pattern: /^SM\s+/gm, replacement: 'static mut ' }
        ];

        // 应用所有转换
        conversions.forEach(({ pattern, replacement }) => {
            rustCode = rustCode.replace(pattern, replacement);
        });

        return rustCode;
    },

    /**
     * 格式化 Nu 代码
     * 添加适当的缩进和空行
     */
    format(nuCode) {
        let lines = nuCode.split('\n');
        let formatted = [];
        let indentLevel = 0;
        const INDENT = '    '; // 4 spaces

        lines.forEach((line, index) => {
            let trimmed = line.trim();
            if (!trimmed) {
                formatted.push('');
                return;
            }

            // 减少缩进的情况
            if (trimmed === '}' || trimmed.startsWith('}')) {
                indentLevel = Math.max(0, indentLevel - 1);
            }

            // 添加缩进
            formatted.push(INDENT.repeat(indentLevel) + trimmed);

            // 增加缩进的情况
            if (trimmed.endsWith('{')) {
                indentLevel++;
            }
        });

        return formatted.join('\n');
    },

    /**
     * 语法高亮（返回带HTML标签的代码）
     * 使用逐行处理，避免正则替换的问题
     */
    highlight(code, lang = 'nu') {
        const lines = code.split('\n');
        const highlightedLines = lines.map(line => {
            // 检查是否是注释行
            if (line.trim().startsWith('//')) {
                return `<span class="token-comment">${this.escapeHtml(line)}</span>`;
            }
            
            // 转义HTML
            let result = this.escapeHtml(line);
            
            if (lang === 'nu') {
                // Nu 语法高亮
                // 字符串
                result = result.replace(/"([^"]*?)"/g, '<span class="token-string">"$1"</span>');
                // 数字
                result = result.replace(/\b(\d+)\b/g, '<span class="token-number">$1</span>');
                // 类型
                result = result.replace(/\b(i32|i64|u32|u64|f32|f64|bool|str|String|V|O|R|A|X|B)\b/g, '<span class="token-type">$1</span>');
                // 关键字
                result = result.replace(/\b(F|f|l|v|S|E|I|M|L|D|C|ST|SM|u|t|wh|a|br|ct)\b/g, '<span class="token-keyword">$1</span>');
            } else {
                // Rust 语法高亮
                // 字符串
                result = result.replace(/"([^"]*?)"/g, '<span class="token-string">"$1"</span>');
                // 数字
                result = result.replace(/\b(\d+)\b/g, '<span class="token-number">$1</span>');
                // 类型
                result = result.replace(/\b(i32|i64|u32|u64|f32|f64|bool|str|String|Vec|Option|Result|Arc|Mutex|Box)\b/g, '<span class="token-type">$1</span>');
                // 关键字
                result = result.replace(/\b(pub|fn|let|mut|struct|enum|impl|match|for|if|return|use|type|where|as|break|continue|mod|const|static)\b/g, '<span class="token-keyword">$1</span>');
            }
            
            return result;
        });
        
        return highlightedLines.join('\n');
    },
    
    /**
     * 转义 HTML 特殊字符
     */
    escapeHtml(text) {
        return text
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;')
            .replace(/'/g, '&#039;');
    },

    /**
     * 生成行号
     */
    generateLineNumbers(code) {
        const lines = code.split('\n').length;
        return Array.from({ length: lines }, (_, i) => `<div>${i + 1}</div>`).join('');
    }
};

// 导出为全局对象
window.NuRustConverter = NuRustConverter;