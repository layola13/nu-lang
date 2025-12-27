// Nu Language Animation - 动画控制器

const AnimationController = {
    // 主时间轴
    mainTimeline: null,
    
    // 初始化
    init() {
        this.mainTimeline = gsap.timeline({
            defaults: { ease: "power2.out" }
        });
        
        // 设置所有场景初始状态
        gsap.set('.scene', { opacity: 0, display: 'none' });
        
        return this;
    },
    
    // Scene 1: 认知的重负 (0-5s)
    animateScene1() {
        const tl = this.mainTimeline;
        
        // 显示场景
        tl.set('#scene1', { display: 'flex', opacity: 1 }, 0);
        
        // 生成代码
        const codeContainer = document.getElementById('code-content1');
        codeContainer.innerHTML = rustCodeData.generateHTML();
        const lines = codeContainer.querySelectorAll('.code-line');
        
        // 代码快速显示
        tl.to(lines, {
            display: 'block',
            opacity: 1,
            duration: 2.5,
            stagger: 0.02,
            ease: "none",
            onUpdate: () => {
                const area = document.getElementById('code-area1');
                area.scrollTop = area.scrollHeight;
            }
        }, 0.5);
        
        // 字幕出现
        tl.to('#subtitle1', {
            opacity: 1,
            scale: 1.1,
            duration: 0.8
        }, 1.0);
        
        tl.to('#subtitle1', {
            opacity: 0,
            duration: 0.5
        }, 2.5);
        
        // 滚动条变化
        tl.to('#fake-scrollbar1', {
            opacity: 1,
            height: '10%',
            duration: 0.1
        }, 0.5);
        
        tl.to('#fake-scrollbar1', {
            height: '5%',
            top: '90%',
            backgroundColor: '#ff4444',
            duration: 2,
            ease: "none"
        }, 1.0);
        
        // 警告出现
        tl.to('#warning1', { opacity: 1, duration: 0 }, 3.5);
        tl.fromTo('.stamp-box',
            { scale: 3, rotation: -45, opacity: 0 },
            { scale: 1, rotation: -15, opacity: 1, duration: 0.3, ease: "elastic.out(1, 0.3)" },
            3.5
        );
        
        // 震动效果
        tl.to('.editor-container', {
            x: 5, y: 5, duration: 0.05,
            repeat: 5, yoyo: true, ease: "rough"
        }, 3.5);
        
        // 背景变红
        tl.to('body', {
            backgroundColor: "#1a0505",
            duration: 1
        }, 3.5);
        
        // 场景淡出
        tl.to('#scene1', {
            opacity: 0,
            duration: 0.5,
            onComplete: () => {
                gsap.set('#scene1', { display: 'none' });
                gsap.set('body', { backgroundColor: '#0d1117' });
            }
        }, 4.5);
        
        return this;
    },
    
    // Scene 2: 压缩的渴望 (5-10s)
    animateScene2() {
        const tl = this.mainTimeline;
        
        // 显示场景
        tl.set('#scene2', { display: 'flex', opacity: 1 }, 5);
        
        // 穿山甲滚入
        tl.fromTo('#pangolin',
            { x: -200, rotation: -360, opacity: 0 },
            { x: 0, rotation: 0, opacity: 1, duration: 1, ease: "bounce.out" },
            5
        );
        
        // 代码块震动
        tl.to('#code-blocks > div', {
            x: '+=10',
            y: '+=10',
            duration: 0.1,
            stagger: 0.05,
            repeat: 3,
            yoyo: true
        }, 7);
        
        // 字幕出现
        tl.to('#subtitle2', {
            opacity: 1,
            y: -20,
            duration: 0.8
        }, 8);
        
        // 场景淡出
        tl.to('#scene2', {
            opacity: 0,
            duration: 0.5,
            onComplete: () => {
                gsap.set('#scene2', { display: 'none' });
            }
        }, 9.5);
        
        return this;
    },
    
    // Scene 3: 核心转化 (10-18s)
    animateScene3() {
        const tl = this.mainTimeline;
        
        // 显示场景
        tl.set('#scene3', { display: 'flex', opacity: 1 }, 10);
        
        // 分屏显示
        tl.fromTo('.split-screen',
            { scaleX: 0 },
            { scaleX: 1, duration: 0.8, stagger: 0.1, transformOrigin: "center" },
            10
        );
        
        // 标题出现
        tl.to('#subtitle3', {
            opacity: 1,
            y: 20,
            duration: 0.8
        }, 10.5);
        
        // Rust 代码显示
        const rustCodeEl = document.getElementById('rust-code');
        codeComparison.rust.forEach((line, i) => {
            const div = document.createElement('div');
            div.className = 'code-line';
            div.innerHTML = line;
            rustCodeEl.appendChild(div);
        });
        
        tl.to('#rust-code .code-line', {
            opacity: 1,
            x: 0,
            duration: 1.5,
            stagger: 0.05,
            ease: "power1.out"
        }, 11);
        
        // 橙色光扫过
        tl.to('#orange-light', {
            opacity: 1,
            duration: 0.3
        }, 13);
        
        tl.to('#orange-light', {
            scaleY: 3,
            duration: 0.5,
            yoyo: true,
            repeat: 1
        }, 13.3);
        
        // Nu 代码显示
        const nuCodeEl = document.getElementById('nu-code');
        codeComparison.nu.forEach((line, i) => {
            const div = document.createElement('div');
            div.className = 'code-line';
            div.innerHTML = line;
            nuCodeEl.appendChild(div);
        });
        
        tl.to('#nu-code .code-line', {
            opacity: 1,
            x: 0,
            duration: 1.5,
            stagger: 0.05,
            ease: "power1.out"
        }, 14);
        
        // 压缩统计
        tl.to('#compression-stats', {
            opacity: 1,
            scale: 1.2,
            duration: 0.8,
            ease: "back.out(1.7)"
        }, 16);
        
        // 场景淡出
        tl.to('#scene3', {
            opacity: 0,
            duration: 0.5,
            onComplete: () => {
                gsap.set('#scene3', { display: 'none' });
            }
        }, 17.5);
        
        return this;
    },
    
    // Scene 4: AI 与速度 (18-25s)
    animateScene4() {
        const tl = this.mainTimeline;
        
        // 显示场景
        tl.set('#scene4', { display: 'flex', opacity: 1 }, 18);
        
        // 管道出现
        tl.to('.pipeline', {
            opacity: 0.5,
            duration: 0.5,
            stagger: 0.1
        }, 18);
        
        // 创建并动画化 Token 粒子
        const container = document.getElementById('token-container');
        tokenData.forEach((token, i) => {
            const particle = document.createElement('div');
            particle.className = 'token-particle';
            particle.textContent = token.text;
            particle.style.left = '0%';
            particle.style.top = `${30 + i * 10}%`;
            container.appendChild(particle);
            
            tl.to(particle, {
                opacity: 1,
                left: '100%',
                duration: 2 + Math.random(),
                ease: "none"
            }, 19 + i * 0.2);
        });
        
        // 标题
        tl.to('#subtitle4', {
            opacity: 1,
            y: -20,
            duration: 0.8
        }, 19);
        
        // 图表动画
        tl.to('#bar1', {
            opacity: 1,
            scaleY: 1,
            duration: 1,
            ease: "elastic.out(1, 0.5)"
        }, 22);
        
        tl.to('#bar2', {
            opacity: 1,
            scaleY: 1,
            duration: 1,
            ease: "elastic.out(1, 0.5)"
        }, 22.3);
        
        // 优势说明
        tl.to('#benefits', {
            opacity: 1,
            y: -10,
            duration: 0.8
        }, 24);
        
        // 场景淡出
        tl.to('#scene4', {
            opacity: 0,
            duration: 0.5,
            onComplete: () => {
                gsap.set('#scene4', { display: 'none' });
            }
        }, 24.5);
        
        return this;
    },
    
    // Scene 5: 最终号召 (25-30s)
    animateScene5() {
        const tl = this.mainTimeline;
        
        // 显示场景
        tl.set('#scene5', { display: 'flex', opacity: 1 }, 25);
        
        // Logo 出现
        tl.to('#logo', {
            opacity: 1,
            scale: 1,
            rotation: 360,
            duration: 1.5,
            ease: "back.out(1.7)"
        }, 25);
        
        // 添加呼吸效果
        tl.add(() => {
            document.querySelector('#logo img').classList.add('breathing');
        }, 26.5);
        
        // 标语出现
        tl.fromTo('#tagline',
            { opacity: 0, y: 30 },
            { opacity: 1, y: 0, duration: 1.5, ease: "power3.out" },
            26.5
        );
        
        // CTA 按钮
        tl.to('#cta', {
            opacity: 1,
            scale: 1,
            duration: 1,
            ease: "elastic.out(1, 0.5)"
        }, 28);
        
        // 按钮脉冲效果
        tl.to('#cta', {
            scale: 1.05,
            duration: 0.5,
            repeat: -1,
            yoyo: true,
            ease: "sine.inOut"
        }, 29);
        
        return this;
    },
    
    // 播放所有动画
    playAll() {
        this.animateScene1()
            .animateScene2()
            .animateScene3()
            .animateScene4()
            .animateScene5();
        
        return this.mainTimeline;
    },
    
    // 重置所有动画
    reset() {
        if (this.mainTimeline) {
            this.mainTimeline.kill();
        }
        this.init();
        return this;
    }
};