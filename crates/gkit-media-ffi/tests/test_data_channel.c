#include "unity.h"
#include "gkit_media.h"
#include <string.h>
#include <stdlib.h>

static void *g_pc;
static void *g_dc;

void setUp(void) {
    g_pc = gkit_media_rtc_create_peer_connection();
    g_dc = gkit_media_rtc_peer_connection_create_data_channel(g_pc, "test-dc");
}

void tearDown(void) {
    if (g_dc) { gkit_media_rtc_destroy_data_channel(g_dc); g_dc = NULL; }
    if (g_pc) { gkit_media_rtc_destroy_peer_connection(g_pc); g_pc = NULL; }
}

void test_create(void) {
    TEST_ASSERT_NOT_NULL(g_dc);
}

void test_label(void) {
    char *label = gkit_media_rtc_data_channel_label(g_dc);
    TEST_ASSERT_NOT_NULL(label);
    TEST_ASSERT_EQUAL_STRING("test-dc", label);
    gkit_media_rtc_free_string(label);
}

void test_send_text(void) {
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_data_channel_send_text(g_dc, "hello"));
}

void test_send_bytes(void) {
    uint8_t data[] = {0, 1, 2, 3};
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_data_channel_send_bytes(g_dc, data, 4));
}

void test_ready_state(void) {
    TEST_ASSERT_EQUAL_INT(1, gkit_media_rtc_data_channel_ready_state(g_dc)); /* Open */
    gkit_media_rtc_data_channel_close(g_dc);
    TEST_ASSERT_EQUAL_INT(3, gkit_media_rtc_data_channel_ready_state(g_dc)); /* Closed */
}

void test_error_on_closed(void) {
    gkit_media_rtc_data_channel_close(g_dc);
    TEST_ASSERT_NOT_EQUAL(0, gkit_media_rtc_data_channel_send_text(g_dc, "after"));
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_create);
    RUN_TEST(test_label);
    RUN_TEST(test_send_text);
    RUN_TEST(test_send_bytes);
    RUN_TEST(test_ready_state);
    RUN_TEST(test_error_on_closed);
    return UNITY_END();
}
