package name.jinleili.wgpu

import android.content.Context
import android.graphics.Canvas
import android.util.AttributeSet
import android.view.SurfaceHolder
import android.view.SurfaceView

class WGPUSurfaceView : SurfaceView, SurfaceHolder.Callback2 {
    private var rustBrige = RustBridge()
    private var rustObj: Long = Long.MAX_VALUE
    private var idx: Int = 1

    constructor(context: Context) : super(context) {
    }

    constructor(context: Context, attrs: AttributeSet) : super(context, attrs) {
    }

    constructor(context: Context, attrs: AttributeSet, defStyle: Int) : super(
        context,
        attrs,
        defStyle
    ) {
    }

    init {
        holder.addCallback(this)
    }


    override fun surfaceChanged(holder: SurfaceHolder, format: Int, width: Int, height: Int) {
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        if (rustObj != Long.MAX_VALUE ){
            rustBrige.dropWgpuCanvas(rustObj)
        }
    }

    override fun surfaceCreated(holder: SurfaceHolder) {
        holder.let { h ->
            rustObj = rustBrige.createWgpuCanvas(h.surface, this.idx)
            setWillNotDraw(false)
        }
    }

    override fun surfaceRedrawNeeded(holder: SurfaceHolder) {
    }

    // API Level 26+
//    override fun surfaceRedrawNeededAsync(holder: SurfaceHolder, drawingFinished: Runnable) {
//        super.surfaceRedrawNeededAsync(holder, drawingFinished)
//    }

    override fun draw(canvas: Canvas?) {
        super.draw(canvas)
        if (rustObj != Long.MAX_VALUE ){
            rustBrige.enterFrame(rustObj)
        }
        invalidate()
    }

    fun changeExample(index: Int) {
        if (rustObj != Long.MAX_VALUE ){
            rustBrige.changeExample(rustObj, index)
            this.idx = index
        }
    }

}