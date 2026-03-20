#include <stdio.h>
#include <stdint.h>

/*
 * 串口绘图解析模板
 *
 * 本模板按 serial_tool 当前解析规则输出文本：
 * 1. CSV 数字格式：
 *    printf("1.23,4.56,7.89\n");
 * 2. key=value 格式：
 *    printf("flag=%u,key=%u,temp=%.2f\n", flag, key, temp);
 *
 * 注意：
 * - 必须以 \n 结尾，串口助手才会按“完整一行”解析并送去绘图。
 * - 每个 value 都必须是数字。
 * - key 建议只用字母、数字、下划线。
 */

static void send_csv_example(float ch1, float ch2, float ch3)
{
    printf("%.2f,%.2f,%.2f\n", ch1, ch2, ch3);
}

static void send_key_value_example(uint32_t flag, uint32_t key, float temp, float hum)
{
    printf("flag=%u,key=%u,temp=%.2f,hum=%.2f\n", flag, key, temp, hum);
}

int main(void)
{
    uint32_t sample = 0;
    uint32_t flag = 100;
    uint32_t key = 0;
    float temp = 25.0f;
    float hum = 60.0f;

    for (;;) {
        /* 模板 1：直接输出多通道 CSV 曲线 */
        send_csv_example(
            1.0f + (float)(sample % 20) * 0.1f,
            2.0f + (float)(sample % 15) * 0.2f,
            3.0f + (float)(sample % 10) * 0.3f
        );

        /* 模板 2：按字段名输出，serial_tool 会把 flag/key/temp/hum 分别画成曲线 */
        send_key_value_example(flag, key, temp, hum);

        sample++;
        flag = 140 + (sample % 8);
        key = sample % 2;
        temp += 0.15f;
        hum += 0.08f;

        if (temp > 30.0f) {
            temp = 25.0f;
        }
        if (hum > 65.0f) {
            hum = 60.0f;
        }

        /*
         * 如果在 MCU/裸机环境里使用：
         * - 请把 printf 重定向到 UART。
         * - 建议每 50ms ~ 500ms 输出一行，便于观察曲线。
         */
    }

    return 0;
}
