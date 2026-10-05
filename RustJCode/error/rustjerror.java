/*
 * RustJ 编译期错误。
 *
 * 做什么：表示前端（词法/语法/语义）与后端（链接/目标文件）在编译过程中发现的、
 *         可预期的用户级错误，例如语法不符、变量未声明、目标文件缺段。
 *
 * 提供什么功能：
 *   - rustjerror(int line, String message)：构造错误；line > 0 时前缀「第 N 行」。
 *   - 继承 RuntimeException，使各阶段无需在方法签名上声明受检异常即可抛出。
 *
 * 为什么独立成包：它是前端与后端共用的横切关注点，独立后两侧都只单向依赖它，
 * 避免 frontend 与 backend 之间产生相互引用。
 */
package error;

public class rustjerror extends RuntimeException {
    /* 以「第 <line> 行: <message>」组装错误信息；line <= 0（如链接期）省略行号前缀。 */
    public rustjerror(int line, String message) {
        super(line > 0 ? "第 " + line + " 行: " + message : message);
    }
}