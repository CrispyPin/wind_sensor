#include <stdint.h>
#include "pico/stdlib.h"
#include "pico/cyw43_arch.h"
#include "pico/cyw43_driver.h"
#include "hardware/adc.h"
#include "lwip/tcp.h"

#include "wifi_cred.h"

#define u32 uint32_t
#define u16 uint16_t
#define u8 uint8_t

// total calibration time. You must manually rotate the sensor around in this time,
// so that every bit gets to read a high and low value
#define CALIBRATION_MS 3000
// milliseconds per measurement in calibration mode
#define CALIBRATION_RESOLUTION 5

// effective time per packet is these two multiplied
#define MEASURE_INTERVAL_MS 100
#define DATA_PER_PACKET 50
// --- end of configuration ---

// #define PACKET_SIZE (DATA_PER_PACKET + PACKET_HEADER_LEN + 1)
#define PACKET_HEADER_LEN 5
const char PACKET_HEADER[PACKET_HEADER_LEN] = "wind:";
const char PACKET_TAIL = '\n';
u8 packet_data[DATA_PER_PACKET];
u32 packet_fill;

char ssid[] = CONFIG_WIFI_SSID;
char pass[] = CONFIG_WIFI_PASSWORD;

const ip_addr_t SERVER_IP = CONFIG_SERVER_IP;
#define SERVER_PORT CONFIG_SERVER_PORT
struct tcp_pcb *tcp_controller;

#define SET_LED(state) cyw43_arch_gpio_put(CYW43_WL_GPIO_LED_PIN, state)

// increasing resolution is not possible without more components and logic, as the pico only has 3 analog inputs
// (you'd also need to make the encoder pattern bigger etc)
#define ENCODER_BITS 3
u8 rotary_encoder_directon;
u8 rotary_encoder_bits;
u8 rotary_encoder_bit[ENCODER_BITS];
u16 rotary_encoder_raw[ENCODER_BITS];
u16 rotary_encoder_min[ENCODER_BITS];
u16 rotary_encoder_max[ENCODER_BITS];
u16 rotary_encoder_high[ENCODER_BITS];
u16 rotary_encoder_low[ENCODER_BITS];
// static const u32 encoder_adc_pins[ENCODER_BITS] = {26, 27, 28};

void update_raw_values() {
	for (int b = 0; b < ENCODER_BITS; b++) {
		adc_select_input(b);
		rotary_encoder_raw[b] = adc_read();
	}
}

void calibrate_brightness() {
	for (int b = 0; b < ENCODER_BITS; b++) {
		rotary_encoder_min[b] = 0xffff;
		rotary_encoder_max[b] = 0;
	}
	for (int i = 0; i < CALIBRATION_MS / CALIBRATION_RESOLUTION; i++) {
		SET_LED(i & 4);
		update_raw_values();
		for (int b = 0; b < ENCODER_BITS; b++) {
			if (rotary_encoder_raw[b] > rotary_encoder_max[b])
				rotary_encoder_max[b] = rotary_encoder_raw[b];
			else if (rotary_encoder_raw[b] < rotary_encoder_min[b])
				rotary_encoder_min[b] = rotary_encoder_raw[b];
		}
		sleep_ms(CALIBRATION_RESOLUTION);
	}
	for (int b = 0; b < ENCODER_BITS; b++) {
		u16 diff = rotary_encoder_max[b] - rotary_encoder_min[b];
		rotary_encoder_low[b] = rotary_encoder_min[b] + diff / 3;
		rotary_encoder_high[b] = rotary_encoder_max[b] - diff / 3;
	}
	SET_LED(1);
	sleep_ms(500);
	SET_LED(0);
}

void update_encoder_value() {
	update_raw_values();
	for (int b = 0; b < ENCODER_BITS; b++) {
		if (rotary_encoder_raw[b] > rotary_encoder_high[b])
			rotary_encoder_bit[b] = 1;
		else if (rotary_encoder_raw[b] < rotary_encoder_low[b])
			rotary_encoder_bit[b] = 0;
	}
	rotary_encoder_bits = 0;
	for (int b = 0; b < ENCODER_BITS; b++) {
		rotary_encoder_bits <<= 1;
		rotary_encoder_bits |= !rotary_encoder_bit[b]; // inverse to make white = 0
	}

	// gray code: 0 1 5 7 3 2 6 4
	// index:     0 1 2 3 4 5 6 7
	// reverse  : 0 1 5 4 7 2 6 3
	const u8 reverse_gray_code[1 << ENCODER_BITS] = {0, 1, 5, 4, 7, 2, 6, 3};
	rotary_encoder_directon = reverse_gray_code[rotary_encoder_bits];
}

static err_t connected_fn(void *arg, struct tcp_pcb *tpcb, err_t err) {
	return ERR_OK;
}

void ensure_server_connection() {
	// TODO fix not able to reconnect
	while (tcp_controller->state != ESTABLISHED) {
		// if (tcp_controller) if(tcp_close(tcp_controller)) SET_LED(1);
		// tcp_controller = tcp_new_ip_type(IPADDR_TYPE_V4);
		int err = tcp_connect(tcp_controller, &SERVER_IP, SERVER_PORT, connected_fn);
		while (err++) {
			SET_LED(1);
			sleep_ms(100);
			SET_LED(0);
			sleep_ms(300);
		}
	}
}

void send_data() {
	ensure_server_connection();
	tcp_write(tcp_controller, PACKET_HEADER, PACKET_HEADER_LEN, 0);
	tcp_write(tcp_controller, packet_data, DATA_PER_PACKET, 0);
	tcp_write(tcp_controller, &PACKET_TAIL, 1, 0);
}


int main() {
	adc_init();
	adc_gpio_init(26);
	adc_gpio_init(27);
	adc_gpio_init(28);

	if (cyw43_arch_init()) {
		SET_LED(1);
		sleep_ms(2000);
		return -1;
	}
	cyw43_arch_enable_sta_mode();

	int err = cyw43_arch_wifi_connect_timeout_ms(ssid, pass, CYW43_AUTH_WPA2_AES_PSK, 10000);
	if (err) {
		PICO_ERROR_BADAUTH; // jump to definition to figure out whats wrong :)
		while (err++) {
			SET_LED(1);
			sleep_ms(100);
			SET_LED(0);
			sleep_ms(100);
		}
		return -1;
	}

	tcp_controller = tcp_new_ip_type(IPADDR_TYPE_V4);
	ensure_server_connection();
	tcp_write(tcp_controller, "hello server :3\n", 19, 0);
	SET_LED(1);
	sleep_ms(200);
	update_raw_values();
	SET_LED(0);

	calibrate_brightness();

	u32 next_measurement_time = time_us_64();
	while (true) {
		sleep_until(next_measurement_time);
		next_measurement_time += MEASURE_INTERVAL_MS * 1000; // millis to micros
		update_encoder_value();
		packet_data[packet_fill++] = rotary_encoder_directon + '0'; // convert number to ascii digit
		if (packet_fill == DATA_PER_PACKET){
			send_data();
			packet_fill = 0;
		}
	}
}
