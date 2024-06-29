PostgreSQL 中数组类型的字段上建 GIN 索引的基本方法

我们用一个例子来说明，假设有一张联系人表：

```SQL
CREATE TABLE contacts(
    id int primary key,
    name varchar(40),
    phone varchar(32)[],
    address text
);
```

由于现在很多人都由多个手机号码，所以我们把手机号码的字段类型建成数组类型。
业务场景是我们需要查询某个号码是那个人的。

我们造一些测试数据：

```SQL
insert into contacts select seq, seq, array[seq+13600000000, seq+13600000001] from generate_series(1, 500000, 2) as seq;
```

然后数组类型的字段 phone 上建 GIN 索引：

```SQL
CREATE INDEX idx_contacts_phone on contacts using gin(phone);
```

注意建 GIN 索引的语法是“using gin(字段名)”。

查询我们用下面的 SQL:

```SQL
SELECT * FROM contacts WHERE phone @> array['13600006688'::varchar(32)];
```

注意上面使用了操作符“@>”，表示包含，右边的表达式也应该是一个于 phone 字段类型相同的数组，所以我们写成“array[‘13600006688’::varchar(32)]”，注意“::varchar(32)”不能省略。

这样我们就查询出号码 13600006688 是谁的号码了。
