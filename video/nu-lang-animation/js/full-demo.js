/**
 * 完整演示动画脚本
 * 展示 Nu → Rust 实时转换 + 格式化 + 编译运行
 */

document.addEventListener('DOMContentLoaded', () => {
    console.log('🎬 Full Demo Animation Loading...');

    // Nu 代码示例（将被逐字输入）
    const nuCodeExample = `// Calculate factorial
F calculate(n: i32) -> i32 {
? n <= 1 {
< 1
}
< n * calculate(n - 1)
}

f main() {
l result = calculate(5);
println!("Result: {}", result);
}`;

    // 格式化后的 Nu 代码
    const nuCodeFormatted = `// Calculate factorial
F calculate(n: i32) -> i32 {
    ? n <= 1 {
        < 1
    }
    < n * calculate(n - 1)
}

f main() {
    l result = calculate(5);
    println!("Result: {}", result);
}`;

    // 元素引用
    const demoTitle = document.getElementById('demo-title');
    const demoSubtitle = document.getElementById('demo-subtitle');
    const nuEditorPanel = document.getElementById('nu-editor-panel');
    const rustEditorPanel = document.getElementById('rust-editor-panel');
    const arrowIndicator = document.getElementById('arrow-indicator');
    const nuCodeContent = document.getElementById('nu-code-content');
    const rustCodeContent = document.getElementById('rust-code-content');
    const nuLineNumbers = document.getElementById('nu-line-numbers');
    const rustLineNumbers = document.getElementById('rust-line-numbers');
    const nuCursor = document.getElementById('nu-cursor');
    const syncIndicator = document.getElementById('sync-indicator');
    const formatBtn = document.getElementById('format-btn');
    const compileBtn = document.getElementById('compile-btn');
    const outputPanel = document.getElementById('output-panel');
    const outputContent = document.getElementById('output-content');
    const nuStatus = document.getElementById('nu-status');
    const rustStatus = document.getElementById('rust-status');

    // GSAP 时间轴
    const tl = gsap.timeline({
        defaults: { ease: "power2.out" }
    });

    // ======== 动画序列 ========

    // 0-1s: 标题出现
    tl.to(demoTitle, { opacity: 1, y: -10, duration: 0.8 }, 0);
    tl.to(demoSubtitle, { opacity: 1, duration: 0.6 }, 0.3);

    // 1-2s: 编辑器面板出现
    tl.to(nuEditorPanel, { opacity: 1, x: 0, duration: 0.8 }, 1);
    tl.to(rustEditorPanel, { opacity: 1, x: 0, duration: 0.8 }, 1.2);
    tl.to(arrowIndicator, { opacity: 1, duration: 0.5 }, 1.5);

    // 2-2.5s: 按钮出现
    tl.to(formatBtn, { opacity: 1, duration: 0.3 }, 2);
    tl.to(compileBtn, { opacity: 1, duration: 0.3 }, 2.2);

    // 2.5-3s: 显示光标
    tl.to(nuCursor, { opacity: 1, duration: 0.2 }, 2.5);

    // 3-10s: 逐字输入 Nu 代码
    let currentCode = '';
    const typingSpeed = 0.05; // 每个字符的时间
    const chars = nuCodeExample.split('');
    
    chars.forEach((char, index) => {
        tl.add(() => {
            currentCode += char;
            const highlighted = NuRustConverter.highlight(currentCode, 'nu');
            nuCodeContent.innerHTML = highlighted;
            
            // 更新行号
            nuLineNumbers.innerHTML = NuRustConverter.generateLineNumbers(currentCode);
            
            // 实时转换为 Rust
            const rustCode = NuRustConverter.convert(currentCode);
            const rustHighlighted = NuRustConverter.highlight(rustCode, 'rust');
            rustCodeContent.innerHTML = rustHighlighted;
            rustLineNumbers.innerHTML = NuRustConverter.generateLineNumbers(rustCode);
            
            // 显示同步指示器
            syncIndicator.style.opacity = '1';
            setTimeout(() => {
                syncIndicator.style.opacity = '0.5';
            }, 100);
            
        }, 3 + index * typingSpeed);
    });

    const typingEndTime = 3 + chars.length * typingSpeed;

    // 输入完成后隐藏光标
    tl.to(nuCursor, { opacity: 0, duration: 0.2 }, typingEndTime);

    // 等待 0.5s
    const formatStartTime = typingEndTime + 0.5;

    // 10s: 格式化按钮高亮
    tl.to(formatBtn, {
        scale: 1.1,
        boxShadow: '0 0 20px rgba(0, 122, 204, 0.8)',
        duration: 0.2,
        yoyo: true,
        repeat: 1
    }, formatStartTime);

    // 10.5s: 开始格式化
    tl.add(() => {
        nuStatus.innerHTML = '<span class="compiling-indicator"></span>Formatting...';
        nuStatus.className = 'status-compiling';
    }, formatStartTime + 0.5);

    // 11s: 应用格式化
    tl.add(() => {
        const formatted = NuRustConverter.format(nuCodeFormatted);
        const highlighted = NuRustConverter.highlight(formatted, 'nu');
        nuCodeContent.innerHTML = highlighted;
        nuLineNumbers.innerHTML = NuRustConverter.generateLineNumbers(formatted);
        
        // 更新 Rust 代码
        const rustCode = NuRustConverter.convert(formatted);
        const rustHighlighted = NuRustConverter.highlight(rustCode, 'rust');
        rustCodeContent.innerHTML = rustHighlighted;
        rustLineNumbers.innerHTML = NuRustConverter.generateLineNumbers(rustCode);
        
        nuStatus.textContent = '✓ Formatted';
        nuStatus.className = 'status-success';
        
        // 高亮效果
        nuCodeContent.classList.add('format-after');
        setTimeout(() => {
            nuCodeContent.classList.remove('format-after');
        }, 1000);
    }, formatStartTime + 1);

    // 12s: 编译按钮高亮
    const compileStartTime = formatStartTime + 2;
    tl.to(compileBtn, {
        scale: 1.1,
        boxShadow: '0 0 20px rgba(255, 136, 0, 0.8)',
        duration: 0.2,
        yoyo: true,
        repeat: 1
    }, compileStartTime);

    // 12.5s: 显示输出面板
    tl.to(outputPanel, { opacity: 1, y: 0, duration: 0.5 }, compileStartTime + 0.5);

    // 13s: 开始编译
    tl.add(() => {
        nuStatus.innerHTML = '<span class="compiling-indicator"></span>Compiling...';
        nuStatus.className = 'status-compiling';
        rustStatus.textContent = 'Compiling...';
        
        outputContent.innerHTML = '<div class="output-line text-blue-400">$ nu2rust example.nu</div>';
    }, compileStartTime + 1);

    // 13.5s: 编译中
    tl.add(() => {
        outputContent.innerHTML += '<div class="output-line text-gray-400">Converting Nu → Rust...</div>';
    }, compileStartTime + 1.5);

    tl.add(() => {
        outputContent.innerHTML += '<div class="output-line text-gray-400">Generating example.rs...</div>';
    }, compileStartTime + 2);

    // 14s: 编译成功
    tl.add(() => {
        outputContent.innerHTML += '<div class="output-line text-green-400">✓ Compilation successful!</div>';
        outputContent.innerHTML += '<div class="output-line text-gray-400">Generated: example.rs</div>';
        outputContent.innerHTML += '<div class="output-line text-gray-400">Size: 156 bytes → 245 bytes</div>';
        
        nuStatus.textContent = '✓ Compiled';
        nuStatus.className = 'status-success';
        rustStatus.textContent = 'Ready to run';
    }, compileStartTime + 2.5);

    // 14.5s: 开始运行
    tl.add(() => {
        outputContent.innerHTML += '<div class="output-line text-blue-400 mt-2">$ cargo run</div>';
        outputContent.innerHTML += '<div class="output-line text-gray-400">   Compiling example v0.1.0</div>';
    }, compileStartTime + 3);

    // 15s: 编译输出
    tl.add(() => {
        outputContent.innerHTML += '<div class="output-line text-gray-400">   Finished dev [unoptimized] target(s) in 0.42s</div>';
        outputContent.innerHTML += '<div class="output-line text-gray-400">    Running `target/debug/example`</div>';
    }, compileStartTime + 3.5);

    // 15.5s: 程序输出
    tl.add(() => {
        outputContent.innerHTML += '<div class="output-line text-green-400 font-bold">Result: 120</div>';
        outputContent.innerHTML += '<div class="output-line text-gray-400 mt-2">Process finished with exit code 0</div>';
        
        rustStatus.textContent = '✓ Executed successfully';
        rustStatus.className = 'text-green-400';
    }, compileStartTime + 4);

    // 16s: 完成提示
    tl.to(demoTitle, {
        textContent: 'Demo Complete! ✨',
        color: '#4ec9b0',
        duration: 0.5
    }, compileStartTime + 4.5);

    tl.to(demoSubtitle, {
        textContent: 'Nu → Rust seamless conversion with VSCode',
        duration: 0.5
    }, compileStartTime + 4.5);

    // 键盘控制
    document.addEventListener('keydown', (e) => {
        if (e.key === ' ') {
            e.preventDefault();
            if (tl.paused()) {
                tl.play();
                console.log('▶️ Playing...');
            } else {
                tl.pause();
                console.log('⏸️ Paused');
            }
        }
        
        if (e.key === 'r' || e.key === 'R') {
            tl.restart();
            currentCode = '';
            nuCodeContent.innerHTML = '';
            rustCodeContent.innerHTML = '';
            outputContent.innerHTML = '';
            console.log('🔄 Restarted');
        }
    });

    // 按钮交互（可选）
    formatBtn.addEventListener('click', () => {
        console.log('Format button clicked');
    });

    compileBtn.addEventListener('click', () => {
        console.log('Compile button clicked');
    });

    console.log('✅ Full Demo Ready!');
    console.log('📝 Controls:');
    console.log('  [Space] - Play/Pause');
    console.log('  [R] - Restart');
});