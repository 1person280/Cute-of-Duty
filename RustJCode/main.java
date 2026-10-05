/* ============================================================================
 *  ⚠️ 临时代码警告（RustJ 一期原型）
 * ============================================================================
 *  本文件是「计划 0003 · RustJ 编译器」的【一期原型】。它唯一的目的是跑通
 *  "Rust 极小子集 -> Java 源码 -> javac -> .class -> 可运行" 的闭环。
 *
 *  ⚠️ 请勿依赖、请勿当作最终架构：
 *    - 它不是真正的 Rust 编译器：无宏系统（只特判 println!）、无借用检查、
 *      无 trait / 泛型 / 模块；产物是 JVM 字节码，而非计划〈2.4〉的原生 exe。
 *    - rust-lld / sysroot / 对象文件 / 增量缓存 一律没有（留待二期）。
 *    - 后续期次会把本原型整体重写（后端从"转译 Java"替换为真实 codegen）。
 *  ⚠️ 任何生产用途一律以 docs/plans/0003-RustJ编译器.md 的终靶为准。
 * ============================================================================
 *
 *  一期支持子集：整数 / 字符串 / 布尔 字面量 · let 变量 · fn 函数 ·
 *  基本表达式（+ - * / 与比较）· println! 宏（含 {} 占位符）。
 *
 *  用法: java -jar RustJ.jar [-RJT=16] [-RJCC=16M] <源文件.rs>
 * ========================================================================= */

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;

/* ---------------------------------------------------------------- 词法单元 */
final class Token {
    enum Kind { IDENT, INT, STRING, PUNCT, EOF }

    final Kind kind;
    final String text;
    final int line;

    Token(Kind kind, String text, int line) {
        this.kind = kind;
        this.text = text;
        this.line = line;
    }

    @Override
    public String toString() {
        return kind + "(" + text + ")@" + line;
    }
}

/* -------------------------------------------------------------------- 错误 */
final class RustJError extends RuntimeException {
    RustJError(int line, String message) {
        super("第 " + line + " 行: " + message);
    }
}

/* -------------------------------------------------------------------- 词法 */
final class Lexer {
    private static final String[] TWO_CHAR = {"->", "==", "!=", "<=", ">=", "&&", "||", "::"};

    private final String src;
    private int pos = 0;
    private int line = 1;

    Lexer(String src) {
        this.src = src;
    }

    List<Token> tokenize() {
        List<Token> out = new ArrayList<>();
        while (true) {
            skipTrivia();
            if (pos >= src.length()) {
                out.add(new Token(Token.Kind.EOF, "", line));
                return out;
            }
            char c = src.charAt(pos);
            if (Character.isLetter(c) || c == '_') {
                out.add(readIdent());
            } else if (Character.isDigit(c)) {
                out.add(readNumber());
            } else if (c == '"') {
                out.add(readString());
            } else {
                out.add(readPunct());
            }
        }
    }

    private void skipTrivia() {
        while (pos < src.length()) {
            char c = src.charAt(pos);
            if (c == '\n') {
                line++;
                pos++;
            } else if (Character.isWhitespace(c)) {
                pos++;
            } else if (c == '/' && pos + 1 < src.length() && src.charAt(pos + 1) == '/') {
                while (pos < src.length() && src.charAt(pos) != '\n') pos++;
            } else {
                return;
            }
        }
    }

    private Token readIdent() {
        int start = pos;
        while (pos < src.length()
                && (Character.isLetterOrDigit(src.charAt(pos)) || src.charAt(pos) == '_')) {
            pos++;
        }
        return new Token(Token.Kind.IDENT, src.substring(start, pos), line);
    }

    private Token readNumber() {
        int start = pos;
        while (pos < src.length() && Character.isDigit(src.charAt(pos))) pos++;
        return new Token(Token.Kind.INT, src.substring(start, pos), line);
    }

    private Token readString() {
        int startLine = line;
        pos++;
        StringBuilder sb = new StringBuilder();
        while (pos < src.length() && src.charAt(pos) != '"') {
            char c = src.charAt(pos++);
            if (c == '\\' && pos < src.length()) {
                char esc = src.charAt(pos++);
                switch (esc) {
                    case 'n': sb.append('\n'); break;
                    case 't': sb.append('\t'); break;
                    case '\\': sb.append('\\'); break;
                    case '"': sb.append('"'); break;
                    default: sb.append(esc);
                }
            } else {
                sb.append(c);
            }
        }
        if (pos >= src.length()) throw new RustJError(startLine, "字符串字面量未闭合");
        pos++;
        return new Token(Token.Kind.STRING, sb.toString(), startLine);
    }

    private Token readPunct() {
        for (String op : TWO_CHAR) {
            if (src.startsWith(op, pos)) {
                pos += op.length();
                return new Token(Token.Kind.PUNCT, op, line);
            }
        }
        return new Token(Token.Kind.PUNCT, String.valueOf(src.charAt(pos++)), line);
    }
}

/* ---------------------------------------------------------- AST：表达式节点 */
final class Expr {
    String kind;
    String text;
    long intVal;
    String strVal;
    boolean boolVal;
    Expr lhs;
    Expr rhs;
    List<Expr> args;

    static Expr intLit(long v) { Expr e = new Expr(); e.kind = "int"; e.intVal = v; return e; }
    static Expr strLit(String v) { Expr e = new Expr(); e.kind = "str"; e.strVal = v; return e; }
    static Expr boolLit(boolean v) { Expr e = new Expr(); e.kind = "bool"; e.boolVal = v; return e; }
    static Expr var(String n) { Expr e = new Expr(); e.kind = "var"; e.text = n; return e; }
    static Expr unary(String op, Expr x) { Expr e = new Expr(); e.kind = "unary"; e.text = op; e.lhs = x; return e; }
    static Expr bin(String op, Expr l, Expr r) { Expr e = new Expr(); e.kind = "bin"; e.text = op; e.lhs = l; e.rhs = r; return e; }
    static Expr call(String n, List<Expr> a) { Expr e = new Expr(); e.kind = "call"; e.text = n; e.args = a; return e; }
    static Expr macro(String n, List<Expr> a) { Expr e = new Expr(); e.kind = "macro"; e.text = n; e.args = a; return e; }
}

/* ------------------------------------------------------------ AST：语句节点 */
final class Stmt {
    String kind;
    String name;
    String typeName;
    boolean mutable;
    Expr expr;
}

/* -------------------------------------------------------------- AST：声明节点 */
final class Param {
    final String name;
    final String type;

    Param(String name, String type) {
        this.name = name;
        this.type = type;
    }
}

final class FnDecl {
    String name;
    List<Param> params;
    String retType;
    List<Stmt> body;
}

/* -------------------------------------------------------------------- 语法 */
final class Parser {
    private final List<Token> toks;
    private int p = 0;

    Parser(List<Token> toks) {
        this.toks = toks;
    }

    List<FnDecl> parseProgram() {
        List<FnDecl> fns = new ArrayList<>();
        while (!peek().kind.equals(Token.Kind.EOF)) fns.add(parseFn());
        return fns;
    }

    private Token peek() { return toks.get(p); }
    private Token next() { return toks.get(p++); }

    private boolean isPunct(String s) {
        Token t = peek();
        return t.kind == Token.Kind.PUNCT && t.text.equals(s);
    }

    private boolean isIdent(String s) {
        Token t = peek();
        return t.kind == Token.Kind.IDENT && t.text.equals(s);
    }

    private void expectPunct(String s) {
        if (!isPunct(s)) throw new RustJError(peek().line, "期望 '" + s + "'，实际 '" + peek().text + "'");
        next();
    }

    private String expectIdent() {
        if (peek().kind != Token.Kind.IDENT) throw new RustJError(peek().line, "期望标识符，实际 '" + peek().text + "'");
        return next().text;
    }

    private FnDecl parseFn() {
        if (!isIdent("fn")) throw new RustJError(peek().line, "一期顶层仅支持 fn 声明");
        next();
        FnDecl fn = new FnDecl();
        fn.name = expectIdent();
        expectPunct("(");
        fn.params = new ArrayList<>();
        if (!isPunct(")")) {
            while (true) {
                String pn = expectIdent();
                expectPunct(":");
                fn.params.add(new Param(pn, expectIdent()));
                if (isPunct(",")) { next(); continue; }
                break;
            }
        }
        expectPunct(")");
        fn.retType = "()";
        if (isPunct("->")) {
            next();
            fn.retType = expectIdent();
        }
        fn.body = parseBlock();
        return fn;
    }

    private List<Stmt> parseBlock() {
        expectPunct("{");
        List<Stmt> stmts = new ArrayList<>();
        while (!isPunct("}")) stmts.add(parseStmt());
        expectPunct("}");
        return stmts;
    }

    private Stmt parseStmt() {
        if (isIdent("let")) return parseLet();
        if (isIdent("return")) {
            next();
            Stmt s = new Stmt();
            s.kind = "return";
            if (!isPunct(";")) s.expr = parseExpr();
            expectPunct(";");
            return s;
        }
        Stmt s = new Stmt();
        s.kind = "expr";
        s.expr = parseExpr();
        expectPunct(";");
        return s;
    }

    private Stmt parseLet() {
        next();
        Stmt s = new Stmt();
        s.kind = "let";
        if (isIdent("mut")) { next(); s.mutable = true; }
        s.name = expectIdent();
        if (isPunct(":")) { next(); s.typeName = expectIdent(); }
        expectPunct("=");
        s.expr = parseExpr();
        expectPunct(";");
        return s;
    }

    private Expr parseExpr() { return parseComparison(); }

    private Expr parseComparison() {
        Expr left = parseAdditive();
        while (isPunct("==") || isPunct("!=") || isPunct("<") || isPunct(">") || isPunct("<=") || isPunct(">=")) {
            String op = next().text;
            left = Expr.bin(op, left, parseAdditive());
        }
        return left;
    }

    private Expr parseAdditive() {
        Expr left = parseMultiplicative();
        while (isPunct("+") || isPunct("-")) {
            String op = next().text;
            left = Expr.bin(op, left, parseMultiplicative());
        }
        return left;
    }

    private Expr parseMultiplicative() {
        Expr left = parseUnary();
        while (isPunct("*") || isPunct("/")) {
            String op = next().text;
            left = Expr.bin(op, left, parseUnary());
        }
        return left;
    }

    private Expr parseUnary() {
        if (isPunct("-") || isPunct("!")) {
            String op = next().text;
            return Expr.unary(op, parseUnary());
        }
        return parsePrimary();
    }

    private Expr parsePrimary() {
        Token t = peek();
        if (t.kind == Token.Kind.INT) { next(); return Expr.intLit(Long.parseLong(t.text)); }
        if (t.kind == Token.Kind.STRING) { next(); return Expr.strLit(t.text); }
        if (isIdent("true")) { next(); return Expr.boolLit(true); }
        if (isIdent("false")) { next(); return Expr.boolLit(false); }
        if (isPunct("(")) { next(); Expr e = parseExpr(); expectPunct(")"); return e; }
        if (t.kind == Token.Kind.IDENT) {
            next();
            if (isPunct("!")) { next(); expectPunct("("); return Expr.macro(t.text, parseArgs()); }
            if (isPunct("(")) { next(); return Expr.call(t.text, parseArgs()); }
            return Expr.var(t.text);
        }
        throw new RustJError(t.line, "无法解析的表达式 '" + t.text + "'");
    }

    private List<Expr> parseArgs() {
        List<Expr> args = new ArrayList<>();
        if (!isPunct(")")) {
            while (true) {
                args.add(parseExpr());
                if (isPunct(",")) { next(); continue; }
                break;
            }
        }
        expectPunct(")");
        return args;
    }
}

/* ------------------------------------------------ 代码生成：Rust AST -> Java 源码 */
final class JavaEmitter {
    private final List<FnDecl> fns;
    private final Map<String, FnDecl> signatures = new HashMap<>();
    private final Map<String, String> varTypes = new HashMap<>();

    JavaEmitter(List<FnDecl> fns) {
        this.fns = fns;
        for (FnDecl f : fns) signatures.put(f.name, f);
    }

    String emit(String className) {
        StringBuilder sb = new StringBuilder();
        sb.append("// 由 RustJ 一期原型自动生成（临时代码，勿手改）\n");
        sb.append("public class ").append(className).append(" {\n");
        for (FnDecl f : fns) {
            varTypes.clear();
            emitFn(sb, f);
        }
        sb.append("}\n");
        return sb.toString();
    }

    private void emitFn(StringBuilder sb, FnDecl f) {
        boolean isMain = f.name.equals("main");
        sb.append("    ").append(isMain ? "public " : "").append("static ")
                .append(isMain ? "void" : mapType(f.retType)).append(" ").append(f.name).append("(");
        boolean first = true;
        for (Param pr : f.params) {
            if (!first) sb.append(", ");
            first = false;
            sb.append(mapType(pr.type)).append(" ").append(pr.name);
            varTypes.put(pr.name, mapType(pr.type));
        }
        if (isMain) {
            if (!first) sb.append(", ");
            sb.append("String[] args");
        }
        sb.append(") {\n");
        for (Stmt s : f.body) emitStmt(sb, s);
        sb.append("    }\n\n");
    }

    private void emitStmt(StringBuilder sb, Stmt s) {
        String ind = "        ";
        if (s.kind.equals("let")) {
            String type = s.typeName != null ? mapType(s.typeName) : infer(s.expr);
            varTypes.put(s.name, type);
            sb.append(ind).append(type).append(" ").append(s.name).append(" = ")
                    .append(emitExpr(s.expr)).append(";\n");
        } else if (s.kind.equals("return")) {
            sb.append(ind).append("return");
            if (s.expr != null) sb.append(" ").append(emitExpr(s.expr));
            sb.append(";\n");
        } else {
            sb.append(ind).append(emitExpr(s.expr)).append(";\n");
        }
    }

    private String emitExpr(Expr e) {
        switch (e.kind) {
            case "int": return Long.toString(e.intVal);
            case "str": return quoted(e.strVal);
            case "bool": return Boolean.toString(e.boolVal);
            case "var": return e.text;
            case "unary": return "(" + e.text + emitExpr(e.lhs) + ")";
            case "bin": return "(" + emitExpr(e.lhs) + " " + e.text + " " + emitExpr(e.rhs) + ")";
            case "call": {
                StringBuilder sb = new StringBuilder(e.text).append("(");
                for (int i = 0; i < e.args.size(); i++) {
                    if (i > 0) sb.append(", ");
                    sb.append(emitExpr(e.args.get(i)));
                }
                return sb.append(")").toString();
            }
            case "macro": return emitMacro(e);
            default: throw new RustJError(0, "内部错误：未知表达式 " + e.kind);
        }
    }

    private String emitMacro(Expr e) {
        if (!e.text.equals("println")) {
            throw new RustJError(0, "一期仅特判 println! 宏，不支持 " + e.text + "!");
        }
        List<Expr> args = e.args;
        if (args.isEmpty()) return "System.out.println()";
        Expr first = args.get(0);
        if (first.kind.equals("str") && first.strVal.contains("{}")) {
            return "System.out.println(" + interpolate(first.strVal, args) + ")";
        }
        if (args.size() == 1) return "System.out.println(" + emitExpr(first) + ")";
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < args.size(); i++) {
            if (i > 0) sb.append(" + \" \" + ");
            sb.append("(").append(emitExpr(args.get(i))).append(")");
        }
        return "System.out.println(" + sb + ")";
    }

    private String interpolate(String template, List<Expr> args) {
        String[] chunks = template.split("\\{\\}", -1);
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < chunks.length; i++) {
            if (!chunks[i].isEmpty()) {
                if (sb.length() > 0) sb.append(" + ");
                sb.append(quoted(chunks[i]));
            }
            if (i < args.size() - 1) {
                if (sb.length() > 0) sb.append(" + ");
                sb.append("(").append(emitExpr(args.get(i + 1))).append(")");
            }
        }
        return sb.length() == 0 ? "\"\"" : sb.toString();
    }

    private String infer(Expr e) {
        switch (e.kind) {
            case "int": return "int";
            case "str": return "String";
            case "bool": return "boolean";
            case "var": return varTypes.getOrDefault(e.text, "int");
            case "call": {
                FnDecl f = signatures.get(e.text);
                return f == null ? "int" : mapType(f.retType);
            }
            case "unary": return e.text.equals("!") ? "boolean" : "int";
            case "bin": return isComparison(e.text) ? "boolean" : "int";
            default: return "int";
        }
    }

    private static boolean isComparison(String op) {
        return op.equals("==") || op.equals("!=") || op.equals("<")
                || op.equals(">") || op.equals("<=") || op.equals(">=");
    }

    private static String mapType(String t) {
        if (t.equals("bool")) return "boolean";
        if (t.equals("i64") || t.equals("isize") || t.equals("usize")) return "long";
        if (t.equals("String") || t.equals("str")) return "String";
        if (t.equals("()")) return "void";
        return "int";
    }

    private static String quoted(String s) {
        StringBuilder sb = new StringBuilder("\"");
        for (char c : s.toCharArray()) {
            switch (c) {
                case '"': sb.append("\\\""); break;
                case '\\': sb.append("\\\\"); break;
                case '\n': sb.append("\\n"); break;
                case '\t': sb.append("\\t"); break;
                default: sb.append(c);
            }
        }
        return sb.append("\"").toString();
    }
}

/* ------------------------------------------------- 工具链：调用外部 javac */
final class Toolchain {
    private Toolchain() {
    }

    static boolean compileJava(Path outDir, Path javaFile) {
        try {
            ProcessBuilder pb = new ProcessBuilder("javac", "-d", outDir.toString(), javaFile.toString());
            pb.inheritIO();
            Process process = pb.start();
            return process.waitFor() == 0;
        } catch (IOException | InterruptedException ex) {
            System.out.println("[RustJ] 调用 javac 失败: " + ex.getMessage());
            if (ex instanceof InterruptedException) Thread.currentThread().interrupt();
            return false;
        }
    }
}

/* ---------------------------------------------------- 入口：CLI 与流程编排 */
public class main {
    public static void main(String[] args) throws Exception {
        System.out.println("RustJ 一期原型（临时代码，仅供闭环验证；详细警告见源码头部）");

        String source = null;
        int threads = 0;
        String cacheChunk = null;
        for (String arg : args) {
            if (arg.startsWith("-RJT=")) {
                threads = parseThreads(arg.substring(5));
            } else if (arg.startsWith("-RJCC=")) {
                cacheChunk = verifyChunk(arg.substring(6));
            } else if (arg.equals("-h") || arg.equals("--help")) {
                printUsage();
                return;
            } else if (!arg.startsWith("-")) {
                source = arg;
            } else {
                System.out.println("[RustJ] 忽略未知参数: " + arg);
            }
        }
        if (threads > 0) System.out.println("[RustJ] -RJT=" + threads + "（一期占位，不生效）");
        if (cacheChunk != null) System.out.println("[RustJ] -RJCC=" + cacheChunk + "（一期占位，不生效）");
        if (source == null) {
            printUsage();
            System.exit(2);
        }

        Path root = Paths.get("").toAbsolutePath();
        Path srcPath = root.resolve(source);
        if (!Files.exists(srcPath)) {
            System.out.println("[RustJ] 找不到源文件: " + srcPath);
            System.exit(2);
        }

        String code = new String(Files.readAllBytes(srcPath), StandardCharsets.UTF_8);
        List<Token> tokens = new Lexer(code).tokenize();
        List<FnDecl> fns = new Parser(tokens).parseProgram();

        String className = classNameOf(srcPath);
        Path outDir = root.resolve("RustJ").resolve("out");
        Files.createDirectories(outDir);

        String javaSource = new JavaEmitter(fns).emit(className);
        Path javaFile = outDir.resolve(className + ".java");
        Files.write(javaFile, javaSource.getBytes(StandardCharsets.UTF_8));
        System.out.println("[RustJ] 生成 Java 源码: " + relative(root, javaFile));

        if (!Toolchain.compileJava(outDir, javaFile)) {
            System.out.println("[RustJ] 编译失败，见上方 javac 输出");
            System.exit(1);
        }
        System.out.println("[RustJ] 产物: " + relative(root, outDir.resolve(className + ".class")));
        System.out.println("[RustJ] 运行: java -cp \"" + relative(root, outDir) + "\" " + className);
    }

    private static int parseThreads(String value) {
        try {
            return Integer.parseInt(value.trim());
        } catch (NumberFormatException ex) {
            System.out.println("[RustJ] -RJT 需为整数，收到: " + value);
            System.exit(2);
            return 0;
        }
    }

    private static String verifyChunk(String value) {
        String chunk = value.trim().replace("\"", "");
        if (!chunk.equals("64K") && !chunk.equals("16M") && !chunk.equals("4G")) {
            System.out.println("[RustJ] -RJCC 仅接受 64K / 16M / 4G，收到: " + value);
            System.exit(2);
        }
        return chunk;
    }

    private static String classNameOf(Path src) {
        String name = src.getFileName().toString();
        int dot = name.lastIndexOf('.');
        if (dot > 0) name = name.substring(0, dot);
        StringBuilder sb = new StringBuilder();
        boolean upperNext = true;
        for (char c : name.toCharArray()) {
            if (Character.isLetterOrDigit(c)) {
                sb.append(upperNext ? Character.toUpperCase(c) : c);
                upperNext = false;
            } else {
                upperNext = true;
            }
        }
        if (sb.length() == 0 || !Character.isLetter(sb.charAt(0))) sb.insert(0, 'P');
        return sb.toString();
    }

    private static String relative(Path root, Path target) {
        try {
            return root.relativize(target).toString().replace('\\', '/');
        } catch (RuntimeException ex) {
            return target.toString();
        }
    }

    private static void printUsage() {
        System.out.println("用法: java -jar RustJ.jar [-RJT=16] [-RJCC=16M] <源文件.rs>");
        System.out.println("  -RJT   并行线程数（一期占位，不生效）");
        System.out.println("  -RJCC  单文件缓存块，仅 64K / 16M / 4G（一期占位，不生效）");
        System.out.println("  -Xms/-Xmx 由 JVM 直接接管，无需 RustJ 处理");
    }
}