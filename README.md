# 充电桩

## 配置文件

程序会尝试加载运行目录下的 `config.toml` 文件，如果不存在文件则会使用默认配置。

默认配置内容如下：

```toml
[price]
path = "prices.json" # 价格文件路径

[charge]
charge_type = "F" # 充电类型，F: 快充, T: 慢充
power = 30.0 # 充电功率，单位为 kW
size = 2 # 充电桩队列长度
tz = "Asia/Shanghai" # 时区
allow_break = false # 是否允许中断充电（是否允许模拟充电桩损坏）
update_interval = 5 # 充电状态更新间隔，单位为秒

[websocket]
url = "ws://localhost:8080/ws" # WebSocket 服务器地址
```

如果想要修改配置文件，可以在运行目录下创建 `config.toml` 文件，只需要写入需要修改的部分即可，程序会自动合并默认配置和用户配置。

## 价格文件

程序会加载配置中价格文件路径中的文件，没有该文件就会在该路径下创建默认的价格文件。

默认价格文件如下

```json
{
  "periods": [
    {
      "start": "00:00:00",
      "end": "07:00:00",
      "price": 0.4
    },
    {
      "start": "07:00:00",
      "end": "10:00:00",
      "price": 0.7
    },
    {
      "start": "10:00:00",
      "end": "15:00:00",
      "price": 1.0
    },
    {
      "start": "15:00:00",
      "end": "18:00:00",
      "price": 0.7
    },
    {
      "start": "18:00:00",
      "end": "21:00:00",
      "price": 1.0
    },
    {
      "start": "21:00:00",
      "end": "23:00:00",
      "price": 0.7
    },
    {
      "start": "23:00:00",
      "end": "00:00:00",
      "price": 0.4
    }
  ],
  "service_fee": 0.8
}
```