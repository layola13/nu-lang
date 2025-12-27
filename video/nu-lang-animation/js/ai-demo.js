/**
 * AI 助手演示脚本 - VSCode 集成版本
 * 展示在 VSCode 中使用 AI 助手，对比 Nu vs Rust 的 Token 效率
 */

// 对话数据
const conversations = [
    {
        user: "请帮我实现一个计算阶乘的函数",
        ai: "我来为您生成代码。使用 Nu 语言可以节省 50% 的 Token！",
        nuCode: `F factorial(n: i32) -> i32 {
    ? n <= 1 {
        < 1
    }
    < n * factorial(n - 1)
}`,
        rustCode: `pub fn factorial(n: i32) -> i32 {
    if n <= 1 {
        return 1;
    }
    return n * factorial(n - 1);
}`,
        nuTokens: 42,
        rustTokens: 85
    },
    {
        user: "添加错误处理和并发支持",
        ai: "已添加完整的错误处理和异步并发支持。注意 Nu 语言仍然保持简洁！",
        nuCode: `u std::sync::A
u tokio::task

F async_factorial(n: i32) -> R<i32, String> {
    ? n < 0 {
        < Err("Negative".into())
    }
    
    l handle = tokio::spawn(async move {
        l mut result = 1;
        L i 1..=n {
            result *= i;
        }
        result
    });
    
    M handle.await {
        Ok(v) => Ok(v),
        Err(e) => Err(e.to_string())
    }
}`,
        rustCode: `use std::sync::Arc;
use tokio::task;

pub fn async_factorial(n: i32) -> Result<i32, String> {
    if n < 0 {
        return Err("Negative".into());
    }
    
    let handle = tokio::spawn(async move {
        let mut result = 1;
        for i in 1..=n {
            result *= i;
        }
        result
    });
    
    match handle.await {
        Ok(v) => Ok(v),
        Err(e) => Err(e.to_string())
    }
}`,
        nuTokens: 78,
        rustTokens: 156
    }
];

/**
 * 打字机效果显示文本
 */
async function typeText(element, text, speed = 30) {
    element.textContent = '';
    for (let i = 0; i < text.length; i++) {
        element.textContent += text[i];
        await new Promise(resolve => setTimeout(resolve, speed));
    }
}

/**
 * 添加用户消息（带打字机效果）
 */
async function addUserMessage(text) {
    const chatContainer = document.getElementById('ai-chat');
    const messageDiv = document.createElement('div');
    messageDiv.className = 'user-message';
    chatContainer.appendChild(messageDiv);
    
    // 动画进入
    await gsap.from(messageDiv, {
        opacity: 0,
        y: 20,
        duration: 0.3
    });
    
    // 打字机效果
    await typeText(messageDiv, text, 40);
    
    // 滚动到底部
    chatContainer.scrollTop = chatContainer.scrollHeight;
}

/**
 * 添加 AI 思考中指示器
 */
function addTypingIndicator() {
    const chatContainer = document.getElementById('ai-chat');
    const messageDiv = document.createElement('div');
    messageDiv.className = 'ai-message';
    messageDiv.id = 'typing-indicator';
    messageDiv.innerHTML = `
        <div class="ai-message-header">
            <div class="ai-icon">🤖</div>
            <span>AI Assistant</span>
        </div>
        <div class="typing-indicator">
            <div class="typing-dot"></div>
            <div class="typing-dot"></div>
            <div class="typing-dot"></div>
        </div>
    `;
    chatContainer.appendChild(messageDiv);
    chatContainer.scrollTop = chatContainer.scrollHeight;
    
    gsap.from(messageDiv, {
        opacity: 0,
        y: 20,
        duration: 0.3
    });
    
    return messageDiv;
}

/**
 * 移除思考指示器
 */
async function removeTypingIndicator() {
    const indicator = document.getElementById('typing-indicator');
    if (indicator) {
        await gsap.to(indicator, {
            opacity: 0,
            duration: 0.2
        });
        indicator.remove();
    }
}

/**
 * 添加 AI 响应消息（仅显示建议，不包含代码执行）
 */
async function addAIResponse(text, nuCode, rustCode, nuTokens, rustTokens) {
    const chatContainer = document.getElementById('ai-chat');
    const messageDiv = document.createElement('div');
    messageDiv.className = 'ai-message';
    
    const headerHTML = `
        <div class="ai-message-header">
            <div class="ai-icon">🤖</div>
            <span>AI Assistant</span>
        </div>
    `;
    
    messageDiv.innerHTML = headerHTML + '<div class="ai-text-content"></div>';
    chatContainer.appendChild(messageDiv);
    
    // 动画进入
    await gsap.from(messageDiv, {
        opacity: 0,
        y: 20,
        duration: 0.3
    });
    
    // 打字机效果显示文本
    const textContent = messageDiv.querySelector('.ai-text-content');
    await typeText(textContent, text, 30);
    
    // 添加代码对比区域（仅展示，不执行）
    const comparisonHTML = `
        <div class="split-comparison">
            <div class="code-block">
                <div class="code-block-header">
                    <span class="text-gray-400 text-xs">Nu Language</span>
                    <span class="token-badge token-nu">${nuTokens} tokens</span>
                </div>
                <pre class="code-line-ai text-gray-300" id="nu-code-suggestion"></pre>
            </div>
            <div class="code-block">
                <div class="code-block-header">
                    <span class="text-gray-400 text-xs">Rust</span>
                    <span class="token-badge token-rust">${rustTokens} tokens</span>
                </div>
                <pre class="code-line-ai text-gray-300" id="rust-code-suggestion"></pre>
            </div>
        </div>
        <div class="savings-badge">
            💰 节省 ${Math.round((1 - nuTokens / rustTokens) * 100)}% Token = 节省成本 & 提升速度
        </div>
        <div style="text-align: center; margin-top: 1rem; color: #4fc3f7; font-size: 0.875rem;">
            ⚡ Applying to editor...
        </div>
    `;
    
    messageDiv.insertAdjacentHTML('beforeend', comparisonHTML);
    chatContainer.scrollTop = chatContainer.scrollHeight;
    
    await new Promise(resolve => setTimeout(resolve, 300));
    
    // 在 AI 对话框中快速显示代码（打字机效果）
    const nuCodeElement = document.getElementById('nu-code-suggestion');
    const rustCodeElement = document.getElementById('rust-code-suggestion');
    
    if (nuCodeElement && rustCodeElement) {
        await Promise.all([
            typeCodeFast(nuCodeElement, nuCode, 'nu'),
            typeCodeFast(rustCodeElement, rustCode, 'rust')
        ]);
    }
    
    chatContainer.scrollTop = chatContainer.scrollHeight;
}

/**
 * 快速打字机效果显示代码（在 AI 对话框中）
 */
async function typeCodeFast(element, code, lang) {
    element.textContent = '';
    const lines = code.split('\n');
    
    for (let i = 0; i < lines.length; i++) {
        if (i > 0) element.textContent += '\n';
        element.textContent += lines[i];
        await new Promise(resolve => setTimeout(resolve, 30));
    }
    
    // 应用语法高亮
    const highlighted = NuRustConverter.highlight(code, lang);
    element.innerHTML = highlighted;
}

/**
 * VSCode Apply Diff 效果 - 在编辑器中逐行应用代码
 */
async function applyDiffInEditor(newCode) {
    const codeContent = document.getElementById('code-content');
    const lineNumbers = document.querySelector('.vscode-line-numbers');
    const newLines = newCode.split('\n');
    
    // 1. 淡出旧代码
    const existingLines = codeContent.querySelectorAll('.code-line');
    if (existingLines.length > 0) {
        await gsap.to(existingLines, {
            opacity: 0.3,
            duration: 0.3
        });
    }
    
    // 2. 清空并准备新行号
    codeContent.innerHTML = '';
    lineNumbers.innerHTML = '';
    
    for (let i = 1; i <= newLines.length; i++) {
        const lineNumDiv = document.createElement('div');
        lineNumDiv.className = 'line-number';
        lineNumDiv.textContent = i;
        lineNumDiv.style.opacity = '0';
        lineNumbers.appendChild(lineNumDiv);
    }
    
    // 3. 逐行插入新代码（apply diff 动画）
    for (let i = 0; i < newLines.length; i++) {
        const lineDiv = document.createElement('div');
        lineDiv.className = 'code-line';
        
        // 绿色背景表示新增
        lineDiv.style.backgroundColor = 'rgba(16, 185, 129, 0.2)';
        lineDiv.style.borderLeft = '3px solid #10b981';
        lineDiv.style.paddingLeft = '0.5rem';
        lineDiv.style.marginLeft = '-0.5rem';
        
        // 应用语法高亮
        const highlighted = NuRustConverter.highlight(newLines[i], 'nu');
        lineDiv.innerHTML = highlighted;
        
        codeContent.appendChild(lineDiv);
        
        // 同时显示行号
        const lineNum = lineNumbers.children[i];
        gsap.to(lineNum, {
            opacity: 1,
            duration: 0.1
        });
        
        // 插入动画
        await gsap.from(lineDiv, {
            opacity: 0,
            x: -40,
            duration: 0.2,
            ease: 'power2.out'
        });
        
        await new Promise(resolve => setTimeout(resolve, 100));
    }
    
    // 4. 移除绿色高亮（淡出效果）
    await new Promise(resolve => setTimeout(resolve, 800));
    const allLines = codeContent.querySelectorAll('.code-line');
    
    for (const line of allLines) {
        await gsap.to(line, {
            backgroundColor: 'transparent',
            borderLeft: 'none',
            marginLeft: '0',
            paddingLeft: '0',
            duration: 0.4
        });
    }
}

/**
 * 主动画流程
 */
async function runAIDemo() {
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // 显示标题
    await gsap.to('#scene-title', {
        opacity: 1,
        y: -20,
        duration: 0.8,
        ease: 'power2.out'
    });
    
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // 淡出标题
    gsap.to('#scene-title', {
        opacity: 0,
        duration: 0.5
    });
    
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // ========== 第一轮对话 ==========
    const conv1 = conversations[0];
    
    // 用户提问（打字机效果）
    await addUserMessage(conv1.user);
    await new Promise(resolve => setTimeout(resolve, 800));
    
    // AI 思考
    addTypingIndicator();
    await new Promise(resolve => setTimeout(resolve, 2000));
    
    // AI 响应（仅在对话框显示建议）
    await removeTypingIndicator();
    await addAIResponse(
        conv1.ai,
        conv1.nuCode,
        conv1.rustCode,
        conv1.nuTokens,
        conv1.rustTokens
    );
    
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // 在 VSCode 编辑器中应用 diff
    await applyDiffInEditor(conv1.nuCode);
    
    await new Promise(resolve => setTimeout(resolve, 2500));
    
    // ========== 第二轮对话 ==========
    const conv2 = conversations[1];
    
    await addUserMessage(conv2.user);
    await new Promise(resolve => setTimeout(resolve, 800));
    
    addTypingIndicator();
    await new Promise(resolve => setTimeout(resolve, 2500));
    
    await removeTypingIndicator();
    await addAIResponse(
        conv2.ai,
        conv2.nuCode,
        conv2.rustCode,
        conv2.nuTokens,
        conv2.rustTokens
    );
    
    await new Promise(resolve => setTimeout(resolve, 1000));
    
    // 在 VSCode 编辑器中应用 diff
    await applyDiffInEditor(conv2.nuCode);
    
    await new Promise(resolve => setTimeout(resolve, 4000));
    
    // 重新开始
    resetAndRestart();
}

/**
 * 重置并重新开始
 */
function resetAndRestart() {
    gsap.to('#vscode-window', {
        opacity: 0,
        duration: 0.5,
        onComplete: () => {
            // 清空聊天
            const chatContainer = document.getElementById('ai-chat');
            chatContainer.innerHTML = '';
            
            // 重置编辑器
            const codeContent = document.getElementById('code-content');
            codeContent.innerHTML = `
                <div class="code-line"><span class="token-comment">// Factorial implementation</span></div>
                <div class="code-line"></div>
                <div class="code-line"><span class="token-comment">// TODO: Add function here</span></div>
                <div class="code-line"></div>
            `;
            
            // 重置行号
            const lineNumbers = document.querySelector('.vscode-line-numbers');
            lineNumbers.innerHTML = `
                <div class="line-number">1</div>
                <div class="line-number">2</div>
                <div class="line-number">3</div>
                <div class="line-number">4</div>
            `;
            
            // 重置标题
            gsap.set('#scene-title', { opacity: 0, y: 0 });
            
            // 淡入
            gsap.to('#vscode-window', {
                opacity: 1,
                duration: 0.5,
                onComplete: () => {
                    setTimeout(() => runAIDemo(), 2000);
                }
            });
        }
    });
}

// 页面加载后启动
window.addEventListener('load', () => {
    setTimeout(() => runAIDemo(), 1000);
});

// 导出
window.AIDemo = {
    run: runAIDemo,
    reset: resetAndRestart
};