#include "unity.h"
#include "gkit_media.h"
#include <stdlib.h>

static void *g_offerer;
static void *g_answerer;

void setUp(void) {
    g_offerer = gkit_media_rtc_create_peer_connection();
    g_answerer = gkit_media_rtc_create_peer_connection();
}

void tearDown(void) {
    gkit_media_rtc_destroy_peer_connection(g_offerer);
    gkit_media_rtc_destroy_peer_connection(g_answerer);
}

void test_create_offer(void) {
    char *sdp = NULL;
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_peer_connection_create_offer(g_offerer, &sdp));
    TEST_ASSERT_NOT_NULL(sdp);
    gkit_media_rtc_free_string(sdp);
}

void test_create_answer(void) {
    char *sdp = NULL;
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_peer_connection_create_answer(g_answerer, &sdp));
    TEST_ASSERT_NOT_NULL(sdp);
    gkit_media_rtc_free_string(sdp);
}

void test_offer_answer_round_trip(void) {
    char *offer_sdp = NULL;
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_peer_connection_create_offer(g_offerer, &offer_sdp));
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_peer_connection_set_local_description(g_offerer, offer_sdp));
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_peer_connection_set_remote_description(g_answerer, offer_sdp));
    gkit_media_rtc_free_string(offer_sdp);

    char *answer_sdp = NULL;
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_peer_connection_create_answer(g_answerer, &answer_sdp));
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_peer_connection_set_local_description(g_answerer, answer_sdp));
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_peer_connection_set_remote_description(g_offerer, answer_sdp));
    gkit_media_rtc_free_string(answer_sdp);
}

void test_ice_candidate(void) {
    TEST_ASSERT_EQUAL_INT(0,
        gkit_media_rtc_peer_connection_add_ice_candidate(
            g_offerer, "candidate:0 1 UDP 2122252543 192.168.1.1 12345 typ host", "0"));
}

void test_ice_state(void) {
    TEST_ASSERT_EQUAL_INT(0, gkit_media_rtc_peer_connection_ice_state(g_offerer));
    gkit_media_rtc_peer_connection_close(g_offerer);
    TEST_ASSERT_EQUAL_INT(6, gkit_media_rtc_peer_connection_ice_state(g_offerer));
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_create_offer);
    RUN_TEST(test_create_answer);
    RUN_TEST(test_offer_answer_round_trip);
    RUN_TEST(test_ice_candidate);
    RUN_TEST(test_ice_state);
    return UNITY_END();
}
