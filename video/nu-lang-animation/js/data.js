// Nu Language Animation - 数据定义

// Scene 1: 生成冗长的 Rust 代码
const rustCodeData = {
    fields: [
        'connection_pool', 'timeout_settings', 'retry_strategy', 'buffer_size',
        'max_connections', 'keep_alive', 'tls_config', 'proxy_settings',
        'user_agent', 'encoding', 'compression', 'metrics_collector'
    ],
    
    generateHTML() {
        let html = '';
        
        // 冗长的 Struct 定义
        html += `<div class="code-line"><span class="kwd">pub struct</span> <span class="typ">SystemConfigurationBuilder</span> {</div>`;
        this.fields.forEach(f => {
            html += `<div class="code-line">    <span class="kwd">pub</span> ${f}: <span class="typ">Option</span>&lt;<span class="typ">u64</span>&gt;,</div>`;
        });
        html += `<div class="code-line">}</div><div class="code-line"><br></div>`;
        
        // 冗长的 Impl 块
        html += `<div class="code-line"><span class="kwd">impl</span> <span class="typ">SystemConfigurationBuilder</span> {</div>`;
        html += `<div class="code-line">    <span class="kwd">pub fn</span> <span class="fn">new</span>() -> <span class="typ">Self</span> {</div>`;
        html += `<div class="code-line">        <span class="typ">Self</span> {</div>`;
        this.fields.forEach(f => {
            html += `<div class="code-line">            ${f}: <span class="kwd">None</span>,</div>`;
        });
        html += `<div class="code-line">        }</div>`;
        html += `<div class="code-line">    }</div><div class="code-line"><br></div>`;
        
        // 部分 Getter/Setter
        this.fields.slice(0, 4).forEach(f => {
            html += `<div class="code-line">    <span class="kwd">pub fn</span> <span class="fn">set_${f}</span>(&<span class="kwd">mut</span> <span class="kwd">self</span>, val: <span class="typ">u64</span>) -> &<span class="kwd">mut</span> <span class="typ">Self</span> {</div>`;
            html += `<div class="code-line">        <span class="kwd">self</span>.${f} = <span class="typ">Some</span>(val);</div>`;
            html += `<div class="code-line">        <span class="kwd">self</span></div>`;
            html += `<div class="code-line">    }</div><div class="code-line"><br></div>`;
        });
        
        html += `<div class="code-line">}</div>`;
        return html;
    }
};

// Scene 3: Rust vs Nu 代码对比
const codeComparison = {
    rust: [
        '<span class="kwd">pub struct</span> <span class="typ">User</span> {',
        '    <span class="kwd">pub</span> name: <span class="typ">String</span>,',
        '    <span class="kwd">pub</span> age: <span class="typ">u32</span>,',
        '}',
        '',
        '<span class="kwd">impl</span> <span class="typ">User</span> {',
        '    <span class="kwd">pub fn</span> <span class="fn">new</span>(name: <span class="typ">String</span>, age: <span class="typ">u32</span>) -> <span class="typ">Self</span> {',
        '        <span class="typ">User</span> { name, age }',
        '    }',
        '',
        '    <span class="kwd">pub fn</span> <span class="fn">greet</span>(&<span class="kwd">self</span>) {',
        '        println!("Hello, {}", <span class="kwd">self</span>.name);',
        '    }',
        '}',
        '',
        '<span class="kwd">fn</span> <span class="fn">main</span>() {',
        '    <span class="kwd">let</span> user = <span class="typ">User</span>::new(',
        '        "Alice".to_string(),',
        '        30',
        '    );',
        '    user.greet();',
        '}'
    ],
    
    nu: [
        '<span class="nu-kwd">S</span> <span class="typ">User</span> {',
        '    name: <span class="typ">String</span>,',
        '    age: <span class="typ">u32</span>,',
        '}',
        '',
        '<span class="nu-kwd">I</span> <span class="typ">User</span> {',
        '    <span class="nu-kwd">F</span> <span class="fn">new</span>(name: <span class="typ">String</span>, age: <span class="typ">u32</span>) -> <span class="typ">Self</span> {',
        '        <span class="typ">User</span> { name, age }',
        '    }',
        '',
        '    <span class="nu-kwd">F</span> <span class="fn">greet</span>(&<span class="kwd">self</span>) {',
        '        println!("Hello, {}", <span class="kwd">self</span>.name);',
        '    }',
        '}',
        '',
        '<span class="nu-kwd">f</span> <span class="fn">main</span>() {',
        '    <span class="nu-kwd">l</span> user = <span class="typ">User</span>::new(',
        '        "Alice".to_string(),',
        '        30',
        '    );',
        '    user.greet();',
        '}'
    ]
};

// Scene 4: Token 数据
const tokenData = [
    { text: 'pub', count: 8 },
    { text: 'fn', count: 12 },
    { text: 'struct', count: 5 },
    { text: 'impl', count: 6 },
    { text: 'let', count: 15 },
    { text: 'mut', count: 10 }
];

// 动画时间轴配置
const timeline = {
    scene1: {
        start: 0,
        end: 5,
        events: {
            typing: { start: 0.5, duration: 2.5 },
            subtitle: { start: 1.0, duration: 1.5 },
            warning: { start: 3.5, duration: 1.5 }
        }
    },
    scene2: {
        start: 5,
        end: 10,
        events: {
            pangolinEnter: { start: 5.0, duration: 1.0 },
            pangolinCollide: { start: 7.0, duration: 0.5 },
            subtitle: { start: 8.0, duration: 2.0 }
        }
    },
    scene3: {
        start: 10,
        end: 18,
        events: {
            split: { start: 10.0, duration: 0.8 },
            rustCode: { start: 11.0, duration: 2.0 },
            lightSweep: { start: 13.0, duration: 1.0 },
            nuCode: { start: 14.0, duration: 2.0 },
            stats: { start: 16.0, duration: 2.0 }
        }
    },
    scene4: {
        start: 18,
        end: 25,
        events: {
            pipelines: { start: 18.0, duration: 1.0 },
            tokens: { start: 19.0, duration: 3.0 },
            chart: { start: 22.0, duration: 2.0 },
            benefits: { start: 24.0, duration: 1.0 }
        }
    },
    scene5: {
        start: 25,
        end: 30,
        events: {
            logo: { start: 25.0, duration: 1.5 },
            tagline: { start: 26.5, duration: 1.5 },
            cta: { start: 28.0, duration: 2.0 }
        }
    }
};