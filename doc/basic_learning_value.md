# 基本学习
`TypedAnt`的语法是类C的.

## 变量声明与定义
> 变量的声明和定义是编程语言中最基础且最常见的操作之一。

### 变量声明
> 变量是用于在内存中存储值的命名空间。

在`TypedAnt`中，我们使用`let`关键字来声明变量，其格式为`let variable_name: Type = initial_value;`。以下是一个示例：

```ant
func foo() void {
    //声明变量 variable 类型为u16, 并指定值为 666
    let variable: u16 = 0;
    variable = 666;
}
```
>提示
目前 TypedAnt 遵循“非必要不使用变量”的原则，即尽可能使用常量。

### 标识符命名
在`TypedAnt`中，标识符必须以字母或下划线开头，
后跟任意字母、数字或下划线，并且不得与关键字重叠。

### 常量
`TypedAnt`使用 const 关键字来声明常量。常量一旦声明并赋值后，其值便不可更改，只能在初次声明时进行赋值。


```ant
extern "C" func printf(s: str, ...) -> i32;
func foo() void {
    const constant: u16 = 666;

    printf("常量 constant 是%d\n", constant);
}
```

> 下一篇`basic_learning_type.md`