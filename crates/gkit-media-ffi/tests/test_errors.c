#include "unity.h"
#include "gkit_media.h"
#include <stdlib.h>

void setUp(void) {}
void tearDown(void) {}

void test_null_handles(void) {
    char *sdp = NULL;
    TEST_ASSERT_NOT_EQUAL(0, gkit_media_rtc_peer_connection_create_offer(NULL, &sdp));
    TEST_ASSERT_NULL(sdp);
    TEST_ASSERT_NOT_EQUAL(0, gkit_media_rtc_peer_connection_close(NULL));
    TEST_ASSERT_EQUAL_INT(-1, gkit_media_rtc_peer_connection_ice_state(NULL));
    TEST_ASSERT_EQUAL_INT(-1, gkit_media_rtc_data_channel_ready_state(NULL));
    TEST_ASSERT_NOT_EQUAL(0, gkit_media_rtc_data_channel_close(NULL));
}

void test_null_destroy_safe(void) {
    gkit_media_rtc_destroy_data_channel(NULL);
    gkit_media_rtc_destroy_peer_connection(NULL);
    gkit_media_rtc_free_string(NULL);
}

void test_closed_peer_rejected(void) {
    void *pc = gkit_media_rtc_create_peer_connection();
    gkit_media_rtc_peer_connection_close(pc);

    char *sdp = NULL;
    TEST_ASSERT_NOT_EQUAL(0, gkit_media_rtc_peer_connection_create_offer(pc, &sdp));
    TEST_ASSERT_NOT_EQUAL(0, gkit_media_rtc_peer_connection_create_answer(pc, &sdp));
    TEST_ASSERT_NULL(gkit_media_rtc_peer_connection_create_data_channel(pc, "dc"));

    gkit_media_rtc_destroy_peer_connection(pc);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_null_handles);
    RUN_TEST(test_null_destroy_safe);
    RUN_TEST(test_closed_peer_rejected);
    return UNITY_END();
}
