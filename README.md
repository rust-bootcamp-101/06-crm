1.Mac上开发gRPC服务使用Nginx做流量转发

配置文件在
```bash
code /opt/homebrew/etc/nginx/nginx.conf
```

添加配置

```
listen       8080;

# 必须开启http2
http2 on;

location / {
    # the 'grpc://' prefix is optional; unencrypted gRPC is the default
    # 将访问 localhost:8080 转发到 grpc://[::1]:50051上
    grpc_pass grpc://[::1]:50051;
}
```

然后重新加载Nginx，再访问 http:localhost:8080

```bash
sudo nginx -s reload

curl http://127.0.0.1:8080
```

退出Nginx
```bash
sudo nginx -s quit
```



2.Mac使用mkcert生成自签的https证书

首先安装
```bash
mkcert -install
```

生成
```bash
mkcert "*.acme.org" localhost 127.0.0.1 ::1
```
