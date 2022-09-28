//
//  libwgpu_on_ios.h
//  wgpu_test
//
//  Created by LiJinlei on 2021/9/10.
//

#ifndef libwgpu_on_app_h
#define libwgpu_on_app_h

#include <stdint.h>

// 这个不透明结构体用来指代 Rust 端的 WgpuCanvas 对象
struct wgpu_canvas;

struct ios_view_obj {
    void *view;
    // CAMetalLayer
    void *metal_layer;
    int maximum_frames;
    void (*callback_to_swift)(int32_t arg);
};

struct wgpu_canvas *create_wgpu_canvas(struct ios_view_obj object);
void enter_frame(struct wgpu_canvas *data);
void change_example(struct wgpu_canvas *data, int32_t index);

#endif /* libwgpu_on_app_h */
