/*
 * 目标架构发射接口。
 *
 * 做什么：把「栈机式求值 + 控制流」所需的最小指令集合抽象成接口，使代码生成
 *   （backend.codegen / backend.eval）与具体硬件架构解耦。
 * 提供什么功能：栈帧建立/拆除、常量与变量槽读写、栈机 push/pop、算术、比较、
 *   标号与相对跳转，以及返回 .text 字节流。
 * 为什么：新增架构（如 arm64）只需再实现本接口一个类，AST 与代码生成均无需改动。
 */
package backend;

public interface arch {
    void begin(int frameBytes);
    void end();

    void movEaxImm(int value);
    void loadEax(int disp);
    void storeEax(int disp);

    void pushEax();
    void popEcx();
    void popEax();
    void movEcxEax();
    void movEaxEdx();

    void addEaxEcx();
    void subEaxEcx();
    void imulEaxEcx();
    void negEax();
    void cdq();
    void idivEcx();

    void cmpEaxEcx();
    void setE();
    void setNE();
    void setL();
    void setLE();
    void setG();
    void setGE();
    void movzxEaxAl();

    int newLabel();
    void bind(int label);
    void jump(int label);
    void branchIfZero(int label);

    byte[] finish();
}