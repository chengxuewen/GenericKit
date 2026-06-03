#include "unity.h"
#include "gkit_media.h"
#include <stdlib.h>

void setUp(void) {}
void tearDown(void) {}

void test_create_destroy(void) {
    void *pc = gkit_media_rtc_create_peer_connection();
    TEST_ASSERT_NOT_NULL(pc);
    gkit_media_rtc_destroy_peer_connection(pc);
}

void test_null_safe_destroy(void) {
    gkit_media_rtc_destroy_peer_connection(NULL);
}

void test_multiple_connections(void) {
    void *pc1 = gkit_media_rtc_create_peer_connection();
    void *pc2 = gkit_media_rtc_create_peer_connection();
    void *pc3 = gkit_media_rtc_create_peer_connection();
    TEST_ASSERT_NOT_NULL(pc1);
    TEST_ASSERT_NOT_NULL(pc2);
    TEST_ASSERT_NOT_NULL(pc3);
    gkit_media_rtc_destroy_peer_connection(pc1);
    gkit_media_rtc_destroy_peer_connection(pc2);
    gkit_media_rtc_destroy_peer_connection(pc3);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_create_destroy);
    RUN_TEST(test_null_safe_destroy);
    RUN_TEST(test_multiple_connections);
    return UNITY_END();
}
