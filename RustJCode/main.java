/*
 * RustJ 二期命令行入口与编译流水线编排。
 *
 * 做什么：串联「读源码 → 词法 → 语法 → 机器码 → COFF → 链接 → PE」全流程，
 *         是整条工具链唯一的对外入口（位于默认包，保持在 RustJCode 根目录）。
 *
 * 提供什么功能：
 *   - main(String[] args)：解析参数（-RJT / -RJCC 暂为占位）、读取 .rs 源文件、
 *     驱动各阶段，并把 .o 与 .exe 写入 RustJ/out/。
 *   - emit(function fn)：把 ast.function 编译为 .text 段字节
 *     （序言 + let 绑定 + 返回表达式 + 尾声）。
 */
import ast.function;
import ast.letstmt;
import backend.coff;
import backend.lld;
import backend.x64;
import frontend.lexer;
import frontend.parser;
import frontend.token;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.List;

public class main {
    public static void main(String[] args) throws Exception {
        String source = null;
        for (String arg : args) {
            if (arg.startsWith("-RJT=")) continue;      // 二期最小链路占位，暂不生效
            if (arg.startsWith("-RJCC=")) continue;     // 二期最小链路占位，暂不生效
            if (arg.startsWith("-")) continue;
            source = arg;
        }
        if (source == null) {
            System.out.println("用法: java -jar RustJ.jar [选项] <源文件.rs>");
            System.exit(2);
        }
        Path root = Paths.get("").toAbsolutePath();
        Path src = root.resolve(source);
        if (!Files.exists(src)) {
            System.out.println("[RustJ] 找不到源文件: " + src);
            System.exit(2);
        }

        String code = new String(Files.readAllBytes(src), StandardCharsets.UTF_8);
        List<token> tokens = new lexer(code).tokenize();
        function fn = new parser(tokens).parse();
        byte[] object = coff.write(fn.name, emit(fn));
        byte[] exe = lld.link(object);

        Path outDir = root.resolve("RustJ").resolve("out");
        Files.createDirectories(outDir);
        Path objPath = outDir.resolve(fn.name + ".o");
        Path exePath = outDir.resolve(fn.name + ".exe");
        Files.write(objPath, object);
        Files.write(exePath, exe);
        System.out.println("[RustJ] 目标文件: " + root.relativize(objPath));
        System.out.println("[RustJ] 可执行文件: " + root.relativize(exePath));
    }

    /* 生成函数体机器码：序言 → let 绑定 → 返回表达式 → 尾声。 */
    private static byte[] emit(function fn) {
        x64 out = new x64();
        out.begin(fn.lets.size() * 4);
        for (letstmt s : fn.lets) {
            s.init.emit(out);                   // 求值初始化表达式 → EAX
            out.storeEax(s.offset);             // 存入变量槽
        }
        fn.body.emit(out);                      // 返回表达式 → EAX
        out.end();
        return out.finish();
    }
}