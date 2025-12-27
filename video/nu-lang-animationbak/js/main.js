// Nu Language Animation - 主入口

document.addEventListener('DOMContentLoaded', () => {
    console.log('🦡 Nu Language Animation Loading...');
    
    // 初始化动画控制器
    const controller = AnimationController.init();
    
    // 播放所有动画
    const timeline = controller.playAll();
    
    // 添加 CTA 按钮事件
    const ctaButton = document.getElementById('cta');
    if (ctaButton) {
        ctaButton.addEventListener('click', () => {
            window.open('https://github.com/layola13/nu-lang', '_blank');
        });
    }
    
    // 添加键盘控制
    document.addEventListener('keydown', (e) => {
        switch(e.key) {
            case ' ': // 空格：暂停/播放
                if (timeline.paused()) {
                    timeline.play();
                    console.log('▶️ Playing...');
                } else {
                    timeline.pause();
                    console.log('⏸️ Paused');
                }
                break;
                
            case 'r': // R：重置
            case 'R':
                controller.reset().playAll();
                console.log('🔄 Reset animation');
                break;
                
            case 'Escape': // ESC：停止
                timeline.pause();
                timeline.progress(0);
                console.log('⏹️ Stopped');
                break;
                
            case 'ArrowRight': // 右箭头：快进 1 秒
                timeline.time(timeline.time() + 1);
                console.log(`⏩ +1s (${timeline.time().toFixed(1)}s)`);
                break;
                
            case 'ArrowLeft': // 左箭头：后退 1 秒
                timeline.time(Math.max(0, timeline.time() - 1));
                console.log(`⏪ -1s (${timeline.time().toFixed(1)}s)`);
                break;
                
            case '1':
            case '2':
            case '3':
            case '4':
            case '5':
                const sceneNum = parseInt(e.key);
                const sceneStart = timeline.labels[`scene${sceneNum}`] || (sceneNum - 1) * 5;
                timeline.seek(sceneStart);
                console.log(`⏭️ Jump to Scene ${sceneNum}`);
                break;
        }
    });
    
    // 调试信息
    if (window.location.search.includes('debug')) {
        enableDebugMode(timeline);
    }
    
    console.log('✅ Animation Ready!');
    console.log('📝 Controls:');
    console.log('  [Space] - Play/Pause');
    console.log('  [R] - Reset');
    console.log('  [Esc] - Stop');
    console.log('  [←/→] - Seek ±1s');
    console.log('  [1-5] - Jump to scene');
});

// 调试模式
function enableDebugMode(timeline) {
    console.log('🐛 Debug Mode Enabled');
    
    // 创建调试面板
    const debugPanel = document.createElement('div');
    debugPanel.style.cssText = `
        position: fixed;
        bottom: 20px;
        right: 20px;
        background: rgba(0,0,0,0.9);
        color: #fff;
        padding: 15px;
        border-radius: 8px;
        font-family: monospace;
        font-size: 12px;
        z-index: 9999;
        min-width: 200px;
        border: 1px solid #ff8800;
    `;
    
    const timeDisplay = document.createElement('div');
    const progressBar = document.createElement('div');
    progressBar.style.cssText = `
        height: 4px;
        background: #333;
        margin: 10px 0;
        border-radius: 2px;
        overflow: hidden;
    `;
    
    const progressFill = document.createElement('div');
    progressFill.style.cssText = `
        height: 100%;
        background: #ff8800;
        width: 0%;
        transition: width 0.1s;
    `;
    progressBar.appendChild(progressFill);
    
    const controlsInfo = document.createElement('div');
    controlsInfo.style.fontSize = '10px';
    controlsInfo.style.marginTop = '10px';
    controlsInfo.style.opacity = '0.7';
    controlsInfo.innerHTML = `
        [Space] Play/Pause<br>
        [R] Reset<br>
        [←/→] Seek<br>
        [1-5] Scenes
    `;
    
    debugPanel.appendChild(timeDisplay);
    debugPanel.appendChild(progressBar);
    debugPanel.appendChild(controlsInfo);
    document.body.appendChild(debugPanel);
    
    // 更新调试信息
    gsap.ticker.add(() => {
        const current = timeline.time();
        const total = timeline.duration();
        const progress = (current / total) * 100;
        
        timeDisplay.textContent = `⏱️ ${current.toFixed(2)}s / ${total.toFixed(2)}s`;
        progressFill.style.width = `${progress}%`;
    });
    
    // 点击进度条跳转
    progressBar.addEventListener('click', (e) => {
        const rect = progressBar.getBoundingClientRect();
        const x = e.clientX - rect.left;
        const percent = x / rect.width;
        timeline.progress(percent);
    });
}

// 性能监控
if (performance && performance.mark) {
    performance.mark('animation-start');
    
    window.addEventListener('load', () => {
        performance.mark('animation-ready');
        performance.measure('animation-load-time', 'animation-start', 'animation-ready');
        
        const measure = performance.getEntriesByName('animation-load-time')[0];
        console.log(`⚡ Load time: ${measure.duration.toFixed(2)}ms`);
    });
}