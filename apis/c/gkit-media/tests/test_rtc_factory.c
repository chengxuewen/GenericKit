#include "unity.h"
#include "gkit_media.h"
#include <stdlib.h>
#include <string.h>

void setUp(void) {}
void tearDown(void) {}

void test_factory_create_default(void) {
    void *f = gkit_media_rtc_create_factory("google_lk");
    // google_lk may not be registered if libwebrtc not available;
    // test that the function doesn't crash
    if (f) {
        const char *name = gkit_media_rtc_factory_backend_name(f);
        TEST_ASSERT_NOT_NULL(name);
        TEST_ASSERT_EQUAL_STRING("google_lk", name);
        gkit_media_rtc_free_string((char *)name);
        gkit_media_rtc_destroy_factory(f);
    }
}

void test_factory_create_from_factory(void) {
    void *f = gkit_media_rtc_create_factory("google_lk");
    if (!f) {
        TEST_IGNORE_MESSAGE("google_lk backend not available");
        return;
    }
    void *pc = gkit_media_rtc_factory_create_peer_connection(f);
    TEST_ASSERT_NOT_NULL(pc);
    gkit_media_rtc_destroy_peer_connection(pc);
    gkit_media_rtc_destroy_factory(f);
}

void test_factory_invalid_name(void) {
    void *f = gkit_media_rtc_create_factory("nonexistent_backend");
    TEST_ASSERT_NULL(f);
}

void test_factory_null_safe_destroy(void) {
    gkit_media_rtc_destroy_factory(NULL);
}

void test_get_registered_backends(void) {
    int count = 0;
    char **names = gkit_media_rtc_get_registered_backends(&count);
    // At least 0 backends; don't crash
    if (names && count > 0) {
        for (int i = 0; i < count; i++) {
            TEST_ASSERT_NOT_NULL(names[i]);
        }
        gkit_media_rtc_free_string_array(names, count);
    }
    // NULL-safe free
    gkit_media_rtc_free_string_array(NULL, 0);
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_factory_create_default);
    RUN_TEST(test_factory_create_from_factory);
    RUN_TEST(test_factory_invalid_name);
    RUN_TEST(test_factory_null_safe_destroy);
    RUN_TEST(test_get_registered_backends);
    return UNITY_END();
}
