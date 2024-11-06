//
//  libwgpu_in_app.h
//
//  Created by LiJinlei on 2021/9/10.
//

#ifndef libwgpu_in_app_h
#define libwgpu_in_app_h

#include <stdint.h>

// 这个不透明结构体用来指代 Rust 端的 WgpuCanvas 对象
typedef struct wgpu_canvas wgpu_canvas_t;

typedef struct {
    void *view;
    void *metal_layer;  // CAMetalLayer
    int32_t maximum_frames;
    void (*callback_to_swift)(int32_t arg);
} ios_view_obj_t;

wgpu_canvas_t* create_wgpu_canvas(ios_view_obj_t object);
void enter_frame(wgpu_canvas_t* canvas);
void change_example(wgpu_canvas_t* canvas, int32_t index);

#endif /* libwgpu_in_app_h */
