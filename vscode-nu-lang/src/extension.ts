import * as vscode from 'vscode';
import { BinaryManager } from './services/binaryManager';
import { ConversionService } from './services/conversionService';
import { SourcemapService } from './services/sourcemapService';
import { CargoService } from './services/cargoService';
import { FormatService } from './services/formatService';
import { BuildService } from './services/buildService';
import { StatusBarManager } from './ui/statusBar';
import { AutoCompileFeature } from './features/autoCompile';
import { ErrorMapperFeature } from './features/errorMapper';
import { NuDebugConfigurationProvider } from './features/debugProvider';
import { BreakpointTranslator } from './features/breakpointTranslator';

let binaryManager: BinaryManager;
let conversionService: ConversionService;
let sourcemapService: SourcemapService;
let cargoService: CargoService;
let formatService: FormatService;
let buildService: BuildService;
let statusBar: StatusBarManager;
let autoCompile: AutoCompileFeature;
let errorMapper: ErrorMapperFeature;
let debugProvider: NuDebugConfigurationProvider;
let breakpointTranslator: BreakpointTranslator;

export async function activate(context: vscode.ExtensionContext) {
    console.log('Nu Language extension is now active');

    // 初始化服务
    try {
        await initializeServices(context);
    } catch (error: any) {
        vscode.window.showErrorMessage(`Failed to initialize Nu extension: ${error.message}`);
        return;
    }

    // 注册命令
    registerCommands(context);

    // 注册调试配置提供器
    registerDebugProvider(context);

    // 初始化断点转换器
    breakpointTranslator = new BreakpointTranslator(sourcemapService);
    context.subscriptions.push(breakpointTranslator);
    
    // 同步现有断点
    await breakpointTranslator.syncExistingBreakpoints();

    // 激活功能
    autoCompile.activate(context);

    // 显示欢迎信息
    const config = vscode.workspace.getConfiguration('nu-lang');
    if (config.get<boolean>('autoCompile', true)) {
        vscode.window.showInformationMessage('Nu Language extension activated with auto-compile enabled');
    }
}

async function initializeServices(context: vscode.ExtensionContext) {
    // 初始化二进制管理器
    binaryManager = new BinaryManager(context);
    await binaryManager.initialize();

    // 验证二进制可用性
    const binaries = await binaryManager.verifyBinaries();
    if (!binaries.nu2rust) {
        vscode.window.showWarningMessage(
            'nu2rust binary not found. Please configure nu-lang.nu2rustPath in settings.'
        );
    }
    if (!binaries.cargo) {
        vscode.window.showWarningMessage(
            'cargo binary not found. Please install Rust or configure nu-lang.cargoPath in settings.'
        );
    }

    // 初始化服务
    conversionService = new ConversionService(binaryManager);
    sourcemapService = new SourcemapService();
    cargoService = new CargoService(binaryManager);
    formatService = new FormatService(binaryManager);
    buildService = new BuildService(binaryManager);

    // 初始化 UI
    statusBar = new StatusBarManager();
    context.subscriptions.push(statusBar);

    // 初始化功能
    errorMapper = new ErrorMapperFeature(cargoService, sourcemapService);
    context.subscriptions.push(errorMapper);

    autoCompile = new AutoCompileFeature(
        conversionService,
        sourcemapService,
        cargoService,
        statusBar,
        async (nuFilePath: string, success: boolean) => {
            // 编译完成回调
            if (success) {
                const rsFilePath = conversionService.getOutputPath(nuFilePath);
                await errorMapper.mapErrorsForFile(nuFilePath, rsFilePath);
            } else {
                errorMapper.clearDiagnostics(nuFilePath);
            }
        }
    );
    context.subscriptions.push(autoCompile);
}

function registerCommands(context: vscode.ExtensionContext) {
    // 注册命令: 编译当前文件
    const compileCommand = vscode.commands.registerCommand(
        'nu-lang.compileFile',
        async () => {
            await autoCompile.compileCurrentFile();
        }
    );

    // 注册命令: 检查 Rust 输出
    const checkCommand = vscode.commands.registerCommand(
        'nu-lang.checkRust',
        async () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor || editor.document.languageId !== 'nu') {
                vscode.window.showWarningMessage('Please open a .nu file first');
                return;
            }

            const nuFilePath = editor.document.uri.fsPath;
            const rsFilePath = conversionService.getOutputPath(nuFilePath);

            // 运行 cargo check
            await vscode.window.withProgress(
                {
                    location: vscode.ProgressLocation.Notification,
                    title: 'Running cargo check...',
                    cancellable: false
                },
                async () => {
                    await errorMapper.mapErrorsForFile(nuFilePath, rsFilePath);
                }
            );
        }
    );

    // 注册命令: 切换自动编译
    const toggleCommand = vscode.commands.registerCommand(
        'nu-lang.toggleAutoCompile',
        () => {
            autoCompile.toggleAutoCompile();
        }
    );

    // 注册命令: 格式化代码
    const formatCommand = vscode.commands.registerCommand(
        'nu-lang.formatFile',
        async () => {
            await formatService.formatCurrentFile();
        }
    );

    // 注册命令: 构建二进制
    const buildCommand = vscode.commands.registerCommand(
        'nu-lang.buildBinary',
        async () => {
            await buildService.buildCurrentFile();
        }
    );

    // 添加到订阅列表
    context.subscriptions.push(
        compileCommand,
        checkCommand,
        toggleCommand,
        formatCommand,
        buildCommand
    );

    // 监听配置变化
    const configListener = vscode.workspace.onDidChangeConfiguration((e) => {
        if (e.affectsConfiguration('nu-lang')) {
            handleConfigurationChange();
        }
    });
    context.subscriptions.push(configListener);
}

function registerDebugProvider(context: vscode.ExtensionContext) {
    // 创建调试配置提供器
    debugProvider = new NuDebugConfigurationProvider(
        binaryManager,
        buildService,
        sourcemapService
    );

    // 注册调试配置提供器
    context.subscriptions.push(
        vscode.debug.registerDebugConfigurationProvider(
            'nu-lang',
            debugProvider
        )
    );
}

async function handleConfigurationChange() {
    // 重新初始化二进制管理器
    if (binaryManager) {
        await binaryManager.initialize();
    }

    // 更新状态栏
    if (statusBar) {
        const config = vscode.workspace.getConfiguration('nu-lang');
        statusBar.setAutoCompileEnabled(config.get<boolean>('autoCompile', true));
    }
}

export function deactivate() {
    console.log('Nu Language extension is now deactivated');
    
    // 清理资源
    if (statusBar) {
        statusBar.dispose();
    }
    if (autoCompile) {
        autoCompile.dispose();
    }
    if (errorMapper) {
        errorMapper.dispose();
    }
    if (breakpointTranslator) {
        breakpointTranslator.dispose();
    }
    if (sourcemapService) {
        sourcemapService.clearCache();
    }
}