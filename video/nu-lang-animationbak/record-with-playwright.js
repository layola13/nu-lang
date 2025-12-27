/**
 * Nu Language Animation 录制脚本
 * 使用 Playwright 自动录制动画为视频
 * 
 * 安装依赖:
 *   npm install -D playwright
 *   npx playwright install chromium
 * 
 * 运行:
 *   node record-with-playwright.js
 */

const { chromium } = require('playwright');
const path = require('path');

async function recordAnimation() {
    console.log('🎬 开始录制 Nu Language 动画...\n');

    // 启动浏览器
    const browser = await chromium.launch({
        headless: false, // 显示浏览器窗口（可以改为 true 进行后台录制）
        args: [
            '--start-maximized',
            '--disable-infobars',
            '--disable-extensions'
        ]
    });

    // 创建上下文并启用视频录制
    const context = await browser.newContext({
        viewport: { 
            width: 1920, 
            height: 1080 
        },
        recordVideo: {
            dir: './videos/',
            size: { 
                width: 1920, 
                height: 1080 
            }
        },
        // 可选：录制高帧率
        // screen: { width: 1920, height: 1080 }
    });

    const page = await context.newPage();

    try {
        // 导航到动画页面
        console.log('📄 加载动画页面...');
        await page.goto('http://localhost:8000', {
            waitUntil: 'networkidle',
            timeout: 10000
        });

        console.log('✅ 页面加载完成');
        console.log('⏱️  等待动画播放 (30秒)...');

        // 等待动画完成
        // Scene 1: 0-5s
        await page.waitForTimeout(5000);
        console.log('   ✓ Scene 1 完成 (认知的重负)');

        // Scene 2: 5-10s
        await page.waitForTimeout(5000);
        console.log('   ✓ Scene 2 完成 (压缩的渴望)');

        // Scene 3: 10-18s
        await page.waitForTimeout(8000);
        console.log('   ✓ Scene 3 完成 (核心转化)');

        // Scene 4: 18-25s
        await page.waitForTimeout(7000);
        console.log('   ✓ Scene 4 完成 (AI 与速度)');

        // Scene 5: 25-30s
        await page.waitForTimeout(5000);
        console.log('   ✓ Scene 5 完成 (最终号召)');

        console.log('\n🎉 动画播放完成！');
        console.log('💾 正在保存视频...');

    } catch (error) {
        console.error('❌ 录制过程中出错:', error.message);
        throw error;
    } finally {
        // 关闭上下文和浏览器（这会触发视频保存）
        await context.close();
        await browser.close();
    }

    console.log('✅ 视频已保存到 ./videos/ 目录');
    console.log('\n📝 后续步骤:');
    console.log('   1. 视频格式为 .webm');
    console.log('   2. 如需转换为 MP4，运行:');
    console.log('      ffmpeg -i videos/*.webm -c:v libx264 -crf 23 nu-animation.mp4');
    console.log('\n🎬 录制完成！');
}

// 错误处理
recordAnimation().catch(error => {
    console.error('\n❌ 录制失败:', error);
    process.exit(1);
});