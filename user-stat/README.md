## 1.PostgreSQL 中数组类型的字段上建 GIN 索引的基本方法

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

## 2.插入百万级别的数据到表中的优化做法

将表中的索引创建放到插入数据后再执行(避免索引影响到插入)，因为索引的数据往往要比原数据更占内存(原因是为了优化查询)
所以，先将数据插入，后再添加索引会更快一些

自己尝试插入 500w 的数据，总耗时不到 10min

1.先执行
sqlx migrate run --target-version 20240628122808(sqlx migrate add 生成的 sql 文件名上的数字)

```bash
$ sqlx migrate run --target-version 20240628122808

Applied 20240628122808/migrate init (5.244292ms)
Skipped 20240629021909/migrate index (0ns)
```

```SQL
CREATE TYPE gender AS ENUM(
    'female',
    'male',
    'unknown'
);

CREATE TABLE IF NOT EXISTS user_stats (
    email VARCHAR(128) NOT NULL PRIMARY KEY,
    name VARCHAR(64) NOT NULL,
    gender gender DEFAULT 'unknown',
    created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP,
    last_visited_at TIMESTAMPTZ,
    last_watched_at TIMESTAMPTZ,
    recent_watched INT[],
    viewed_but_not_started INT[],
    started_but_not_finished INT[],
    finished INT[],
    last_email_notification TIMESTAMPTZ,
    last_in_app_notification TIMESTAMPTZ,
    last_sms_notification TIMESTAMPTZ
);
```

2.插入数据

3.添加索引(由于插入的数据量很大，后续建索引也是比较费时间的，但相比于先建索引再插入数据，这个时间是快很多的)
sqlx migrate run --target-version 20240629021909(或者 sqlx migrate run)

```bash
$ sqlx migrate run
Applied 20240629021909/migrate index (240.850905708s)
```

```SQL

CREATE INDEX user_stats_created_at_idx ON user_stats(created_at);
CREATE INDEX user_stats_last_visited_at_idx ON user_stats(last_visited_at);
CREATE INDEX user_stats_last_watched_at_idx ON user_stats(last_watched_at);

CREATE INDEX user_stats_recent_watched_idx ON user_stats USING GIN(recent_watched);
CREATE INDEX user_stats_viewed_but_not_started_idx ON user_stats USING GIN(viewed_but_not_started);
CREATE INDEX user_stats_started_but_not_finished_idx ON user_stats USING GIN(started_but_not_finished);


CREATE INDEX user_stats_last_email_notification_idx ON user_stats(last_email_notification);
CREATE INDEX user_stats_last_in_app_notification_idx ON user_stats(last_in_app_notification);
CREATE INDEX user_stats_last_sms_notification_idx ON user_stats(last_sms_notification);
```
