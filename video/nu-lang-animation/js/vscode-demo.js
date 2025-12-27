/**
 * VSCode 演示动画脚本
 * 展示 Nu Language 的 VSCode 集成功能
 */

document.addEventListener('DOMContentLoaded', () => {
    console.log('🎬 VSCode Demo Animation Loading...');
    
    // GSAP 时间轴
    const tl = gsap.timeline({
        defaults: { ease: "power2.out" }
    });

    // 0-2s: VSCode 窗口淡入
    tl.to('#scene-vscode', {
        display: 'flex',
        opacity: 1,
        duration: 0
    }, 0);

    tl.to('#vscode-window', {
        opacity: 1,
        scale: 1,
        duration: 1,
        ease: "back.out(1.2)"
    }, 0.5);

    // 1-1.5s: 标题出现
    tl.to('#vscode-title', {
        opacity: 1,
        y: -10,
        duration: 0.8
    }, 1);

    // 2-3s: 代码行打字效果（模拟）
    tl.to('.code-line', {
        opacity: 1,
        x: 0,
        duration: 0.8,
        stagger: 0.1
    }, 2);

    // 3-4s: 鼠标移动到第4行（模拟）
    tl.add(() => {
        // 高亮第4行
        const line4 = document.querySelector('[data-line="4"]');
        if (line4) {
            line4.style.background = 'rgba(255,255,255,0.1)';
        }
    }, 3);

    // 4-5s: 显示断点
    tl.to('#breakpoint-marker', {
        opacity: 1,
        scale: 1,
        duration: 0.3,
        ease: "back.out(2)"
    }, 4);

    // 5-6s: 右键菜单出现
    tl.to('#context-menu', {
        opacity: 1,
        scale: 1,
        duration: 0.3,
        ease: "back.out(1.5)"
    }, 5);

    // 6-6.5s: 高亮"Compile Nu File"选项
    tl.add(() => {
        const compileItem = document.querySelector('[data-action="compile"]');
        if (compileItem) {
            compileItem.style.background = '#094771';
            compileItem.style.color = 'white';
        }
    }, 6);

    // 6.5-7s: 点击编译
    tl.to('#context-menu', {
        opacity: 0,
        duration: 0.2
    }, 6.5);

    // 7-8s: 显示编译中提示
    tl.to('#compile-toast', {
        opacity: 1,
        x: 0,
        duration: 0.3
    }, 7);

    // 7.5s: 更新状态栏
    tl.add(() => {
        const statusText = document.getElementById('status-text');
        if (statusText) {
            statusText.textContent = 'Compiling...';
        }
    }, 7.5);

    // 8.5-9s: 编译完成
    tl.to('#compile-toast', {
        opacity: 0,
        duration: 0.2
    }, 8.5);

    tl.to('#compile-success', {
        opacity: 1,
        x: 0,
        duration: 0.3
    }, 8.7);

    // 更新状态栏为已编译
    tl.add(() => {
        const statusText = document.getElementById('status-text');
        const compileStatus = document.getElementById('compile-status');
        if (statusText) {
            statusText.textContent = 'Nu Language';
        }
        if (compileStatus) {
            compileStatus.style.opacity = '1';
        }
    }, 8.7);

    // 9.5-10s: 编译成功提示消失
    tl.to('#compile-success', {
        opacity: 0,
        duration: 0.3
    }, 9.5);

    // 10-11s: F5 提示出现
    tl.to('#f5-hint', {
        opacity: 1,
        scale: 1,
        duration: 0.5,
        ease: "back.out(1.5)"
    }, 10);

    // 11.5s: 模拟按下 F5
    tl.add(() => {
        const f5Key = document.querySelector('.keyboard-key');
        if (f5Key) {
            gsap.to(f5Key, {
                scale: 0.9,
                duration: 0.1,
                yoyo: true,
                repeat: 1
            });
        }
    }, 11.5);

    // 12s: F5 提示消失
    tl.to('#f5-hint', {
        opacity: 0,
        duration: 0.3
    }, 12);

    // 12.5-13s: 调试工具栏出现
    tl.to('#debug-toolbar', {
        opacity: 1,
        y: 0,
        duration: 0.5,
        ease: "back.out(1.5)"
    }, 12.5);

    // 13-13.5s: 调试高亮行出现（停在断点处）
    tl.to('#debug-highlight', {
        opacity: 1,
        duration: 0.3
    }, 13);

    // 13.5s: 状态栏更新为调试模式
    tl.add(() => {
        const statusBar = document.querySelector('.vscode-statusbar');
        if (statusBar) {
            statusBar.style.background = '#f48771'; // 调试模式颜色
        }
        const statusText = document.getElementById('status-text');
        if (statusText) {
            statusText.textContent = '⏸️ Paused on breakpoint';
        }
    }, 13.5);

    // 14-15s: 调试动作演示（Step Over）
    tl.add(() => {
        // 高亮 Step Over 按钮
        const stepBtn = document.querySelectorAll('.debug-btn')[3];
        if (stepBtn) {
            gsap.to(stepBtn, {
                background: '#007ACC',
                scale: 1.1,
                duration: 0.2,
                yoyo: true,
                repeat: 1
            });
        }
    }, 14);

    // 15s: 调试高亮移动到下一行
    tl.to('#debug-highlight', {
        top: '142px', // 移动到第5行
        duration: 0.5,
        ease: "power2.inOut"
    }, 15);

    // 16-17s: 继续执行（Play按钮）
    tl.add(() => {
        const playBtn = document.querySelectorAll('.debug-btn')[0];
        if (playBtn) {
            gsap.to(playBtn, {
                background: '#007ACC',
                scale: 1.1,
                duration: 0.2,
                yoyo: true,
                repeat: 1
            });
        }
    }, 16);

    // 17s: 调试结束
    tl.to('#debug-highlight', {
        opacity: 0,
        duration: 0.3
    }, 17);

    tl.to('#debug-toolbar', {
        opacity: 0,
        y: 20,
        duration: 0.3
    }, 17.2);

    // 17.5s: 状态栏恢复正常
    tl.add(() => {
        const statusBar = document.querySelector('.vscode-statusbar');
        if (statusBar) {
            statusBar.style.background = '#007ACC';
        }
        const statusText = document.getElementById('status-text');
        if (statusText) {
            statusText.textContent = 'Nu Language';
        }
    }, 17.5);

    // 18-19s: 完成提示
    tl.to('#vscode-title', {
        textContent: 'Debug Complete! ✨',
        duration: 0
    }, 18);

    tl.to('#vscode-title', {
        scale: 1.1,
        duration: 0.3,
        yoyo: true,
        repeat: 1
    }, 18);

    // 添加键盘事件监听（可选的交互）
    document.addEventListener('keydown', (e) => {
        if (e.key === 'F5') {
            e.preventDefault();
            console.log('F5 pressed - Starting debug...');
            // 可以添加重新播放调试动画的逻辑
        }
        
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
            console.log('🔄 Restarted');
        }
    });

    // 鼠标悬停交互（可选）
    const codeLines = document.querySelectorAll('.code-line');
    codeLines.forEach((line, index) => {
        line.addEventListener('mouseenter', () => {
            line.style.background = 'rgba(255,255,255,0.05)';
        });
        line.addEventListener('mouseleave', () => {
            if (!line.classList.contains('breakpoint-line')) {
                line.style.background = '';
            }
        });
    });

    console.log('✅ VSCode Demo Ready!');
    console.log('📝 Controls:');
    console.log('  [Space] - Play/Pause');
    console.log('  [R] - Restart');
    console.log('  [F5] - Trigger debug (in full version)');
});