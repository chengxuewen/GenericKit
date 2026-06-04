#include <gtest/gtest.h>
#include <gkit_media_video_frame.hpp>

TEST(VideoFrame, CreateDestroy) {
    gkit::VideoFrame vf(320, 240);
    ASSERT_TRUE(vf.valid());
    EXPECT_EQ(vf.width(), 320u);
    EXPECT_EQ(vf.height(), 240u);
}

TEST(VideoFrame, CreateNV12) {
    auto vf = gkit::VideoFrame::create_nv12(100, 100);
    ASSERT_TRUE(vf.valid());
    EXPECT_EQ(vf.buffer_type(), 5); // NV12 = 5
}

TEST(VideoFrame, Rotation) {
    gkit::VideoFrame vf(10, 10);
    EXPECT_EQ(vf.rotation(), 0);
    vf.set_rotation(90);
    EXPECT_EQ(vf.rotation(), 90);
    vf.set_rotation(270);
    EXPECT_EQ(vf.rotation(), 270);
}

TEST(VideoFrame, Timestamp) {
    gkit::VideoFrame vf(10, 10);
    EXPECT_EQ(vf.timestamp_us(), 0);
    vf.set_timestamp_us(123456789);
    EXPECT_EQ(vf.timestamp_us(), 123456789);
}

TEST(VideoFrame, GetI420Planes) {
    gkit::VideoFrame vf(16, 16);
    std::vector<uint8_t> y, u, v;
    uint32_t sy, su, sv;
    ASSERT_TRUE(vf.get_i420_planes(y, sy, u, su, v, sv));
    EXPECT_EQ(sy, 16u);
    EXPECT_EQ(su, 8u);
    EXPECT_EQ(sv, 8u);
}

TEST(VideoFrame, Scale) {
    gkit::VideoFrame vf(64, 64);
    auto scaled = vf.scale(32, 32);
    ASSERT_TRUE(scaled.valid());
    EXPECT_EQ(scaled.width(), 32u);
    EXPECT_EQ(scaled.height(), 32u);
}

TEST(VideoFrame, Crop) {
    gkit::VideoFrame vf(64, 64);
    auto cropped = vf.crop(8, 8, 32, 32);
    ASSERT_TRUE(cropped.valid());
    EXPECT_EQ(cropped.width(), 32u);
    EXPECT_EQ(cropped.height(), 32u);
}

TEST(VideoFrame, Rotate) {
    gkit::VideoFrame vf(32, 16);
    auto rotated = vf.rotate(90);
    ASSERT_TRUE(rotated.valid());
    EXPECT_EQ(rotated.width(), 16u);
    EXPECT_EQ(rotated.height(), 32u);
}

TEST(VideoFrame, MoveSemantics) {
    gkit::VideoFrame vf(16, 16);
    auto vf2 = std::move(vf);
    EXPECT_FALSE(vf.valid());
    EXPECT_TRUE(vf2.valid());
    EXPECT_EQ(vf2.width(), 16u);
}

int main(int argc, char** argv) {
    ::testing::InitGoogleTest(&argc, argv);
    return RUN_ALL_TESTS();
}
