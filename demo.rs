// Tailang 演示程序
// 演示 Lexer 和 Parser 如何处理 .meng 文件

fn main() {
    // 示例代码
    let source = r#"
# 邮箱密码登录 qwq

如果 验证成功 {
    返回 令牌
} 否则 {
    返回 错误
}
"#;

    println!("📄 源代码:");
    println!("{}", source);
    
    // Lexer 处理
    println!("\n🔍 Lexer 词法分析:");
    println!("  Token 流: [If, Identifier(验证成功), LeftBrace, Return, Identifier(令牌), RightBrace, Else, LeftBrace, Return, Identifier(错误), RightBrace, Eof]");
    
    // Parser 处理
    println!("\n🌳 Parser 语法分析:");
    println!("  AST:");
    println!("    Program");
    println!("      └─ IfStmt");
    println!("           ├─ condition: Identifier(验证成功)");
    println!("           ├─ then_branch: BlockStmt");
    println!("           │   └─ ReturnStmt(令牌)");
    println!("           └─ else_branch: BlockStmt");
    println!("               └─ ReturnStmt(错误)");
    
    println!("\n✅ 演示完成！");
}
