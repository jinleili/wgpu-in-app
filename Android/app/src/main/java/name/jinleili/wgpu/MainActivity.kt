package name.jinleili.wgpu

import androidx.appcompat.app.AppCompatActivity
import android.os.Bundle
import androidx.core.view.children
import com.google.android.material.button.MaterialButtonToggleGroup

class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContentView(R.layout.activity_main)
        var surfaceV = findViewById<WGPUSurfaceView>(R.id.wgpuSurfaceView)
        var toggleBt = findViewById<MaterialButtonToggleGroup>(R.id.toggleBt)
        toggleBt.isSingleSelection = true
        toggleBt.isSelectionRequired = true
        toggleBt.addOnButtonCheckedListener { toggleButton, checkedId, isChecked ->
            if (isChecked) {
                var idx = 0
                for (item in toggleButton.children) {
                    if (item.id == checkedId) {
                        surfaceV.changeExample(idx)
                        break
                    }
                    idx += 1
                }
            }
        }
    }

}