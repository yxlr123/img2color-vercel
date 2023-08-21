# img2color
本项目使用Rust编写，具有较高的性能，~~应该吧~~
新螃蟹🦀的小练习，代码风格可能比较**混乱**

> 项目完善程度不高

## 部署
[![Deploy with Vercel](https://vercel.com/button)](https://vercel.com/new/clone?repository-url=https://github.com/yxlr123/img2color-vercel/)

### 环境变量
| 变量名               | 说明                                |
|----------------------|-------------------------------------|
| USERNAME             | (必填) redis用户名                  |
| REDIS_HOST           |（必填）redis数据库地址(不带redis://)|
| PASSWORD             |（必填）redis数据库密码              |
| REDIS_PORT           |（必填）redis数据库的端口            |

## api

只有一个~ `/api/img2color`

参数：

| 参数                  | 说明                                |
|----------------------|--------------------------------------|
| img                  | (必填) 需要提取主题色的图片URL       |

返回示例：

``` json
{
    rgb: "#BC695A",
}
```

说明：

| 返回值                   | 说明                                 |
|-------------------------|--------------------------------------|
| error                   | 错误 （nil/string）     可信度不高      |
| rgb                     | 主题色Hex（string）                    |              
