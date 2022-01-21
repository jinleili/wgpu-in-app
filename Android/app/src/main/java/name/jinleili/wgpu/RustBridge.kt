package name.jinleili.wgpu

import android.view.Surface

class RustBridge {
    init {
        System.loadLibrary("wgpu_on_app")
    }

    external fun createWgpuCanvas(surface: Surface): Long
    external fun enterFrame(rustObj: Long)
    external fun dropWgpuCanvas(rustObj: Long)

}