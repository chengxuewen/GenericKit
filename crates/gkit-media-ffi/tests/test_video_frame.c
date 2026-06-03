#include "unity.h"
#include "gkit_media.h"
#include <stdlib.h>
#include <string.h>

void setUp(void) {}
void tearDown(void) {}

void test_create_destroy(void) {
    void *vf = gkit_media_video_frame_create(320, 240);
    TEST_ASSERT_NOT_NULL(vf);
    gkit_media_video_frame_destroy(vf);
}

void test_get_dimensions(void) {
    void *vf = gkit_media_video_frame_create(640, 480);
    TEST_ASSERT_EQUAL_UINT(640, gkit_media_video_frame_get_width(vf));
    TEST_ASSERT_EQUAL_UINT(480, gkit_media_video_frame_get_height(vf));
    gkit_media_video_frame_destroy(vf);
}

void test_create_nv12(void) {
    void *vf = gkit_media_video_frame_create_nv12(100, 100);
    TEST_ASSERT_NOT_NULL(vf);
    TEST_ASSERT_EQUAL_INT(5, gkit_media_video_frame_get_buffer_type(vf)); /* NV12 */
    gkit_media_video_frame_destroy(vf);
}

void test_rotation(void) {
    void *vf = gkit_media_video_frame_create(10, 10);
    TEST_ASSERT_EQUAL_INT(0, gkit_media_video_frame_get_rotation(vf));
    gkit_media_video_frame_set_rotation(vf, 90);
    TEST_ASSERT_EQUAL_INT(90, gkit_media_video_frame_get_rotation(vf));
    gkit_media_video_frame_set_rotation(vf, 270);
    TEST_ASSERT_EQUAL_INT(270, gkit_media_video_frame_get_rotation(vf));
    gkit_media_video_frame_destroy(vf);
}

void test_timestamp(void) {
    void *vf = gkit_media_video_frame_create(10, 10);
    TEST_ASSERT_EQUAL_INT64(0, gkit_media_video_frame_get_timestamp(vf));
    gkit_media_video_frame_set_timestamp(vf, 123456789);
    TEST_ASSERT_EQUAL_INT64(123456789, gkit_media_video_frame_get_timestamp(vf));
    gkit_media_video_frame_destroy(vf);
}

void test_get_i420_planes(void) {
    void *vf = gkit_media_video_frame_create(16, 16);
    uint8_t data_y[512], data_u[256], data_v[256];
    uint32_t stride_y, stride_u, stride_v;
    memset(data_y, 0, sizeof(data_y));
    memset(data_u, 0, sizeof(data_u));
    memset(data_v, 0, sizeof(data_v));

    int rc = gkit_media_video_frame_get_i420_planes(
        vf, data_y, &stride_y, data_u, &stride_u, data_v, &stride_v);
    TEST_ASSERT_EQUAL_INT(0, rc);
    TEST_ASSERT_EQUAL_UINT(16, stride_y);
    TEST_ASSERT_EQUAL_UINT(8, stride_u);
    gkit_media_video_frame_destroy(vf);
}

void test_scale(void) {
    void *vf = gkit_media_video_frame_create(64, 64);
    void *scaled = gkit_media_video_frame_scale(vf, 32, 32);
    TEST_ASSERT_NOT_NULL(scaled);
    TEST_ASSERT_EQUAL_UINT(32, gkit_media_video_frame_get_width(scaled));
    TEST_ASSERT_EQUAL_UINT(32, gkit_media_video_frame_get_height(scaled));
    gkit_media_video_frame_destroy(vf);
    gkit_media_video_frame_destroy(scaled);
}

void test_crop(void) {
    void *vf = gkit_media_video_frame_create(64, 64);
    void *cropped = gkit_media_video_frame_crop(vf, 8, 8, 32, 32);
    TEST_ASSERT_NOT_NULL(cropped);
    TEST_ASSERT_EQUAL_UINT(32, gkit_media_video_frame_get_width(cropped));
    gkit_media_video_frame_destroy(vf);
    gkit_media_video_frame_destroy(cropped);
}

void test_rotate(void) {
    void *vf = gkit_media_video_frame_create(32, 16);
    void *rotated = gkit_media_video_frame_rotate(vf, 90);
    TEST_ASSERT_NOT_NULL(rotated);
    TEST_ASSERT_EQUAL_UINT(16, gkit_media_video_frame_get_width(rotated));
    TEST_ASSERT_EQUAL_UINT(32, gkit_media_video_frame_get_height(rotated));
    gkit_media_video_frame_destroy(vf);
    gkit_media_video_frame_destroy(rotated);
}

void test_null_safe(void) {
    gkit_media_video_frame_destroy(NULL);
    TEST_ASSERT_EQUAL_UINT(0, gkit_media_video_frame_get_width(NULL));
    TEST_ASSERT_NULL(gkit_media_video_frame_scale(NULL, 10, 10));
}

int main(void) {
    UNITY_BEGIN();
    RUN_TEST(test_create_destroy);
    RUN_TEST(test_get_dimensions);
    RUN_TEST(test_create_nv12);
    RUN_TEST(test_rotation);
    RUN_TEST(test_timestamp);
    RUN_TEST(test_get_i420_planes);
    RUN_TEST(test_scale);
    RUN_TEST(test_crop);
    RUN_TEST(test_rotate);
    RUN_TEST(test_null_safe);
    return UNITY_END();
}
