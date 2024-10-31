package name.jinleili.wgpu

import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.foundation.layout.*
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalConfiguration
import androidx.compose.ui.res.colorResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.compose.ui.viewinterop.AndroidView
import name.jinleili.wgpu.ui.theme.MyApplicationTheme

class MainActivity : ComponentActivity() {

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        setContent {
            MyApplicationTheme {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = colorResource(id = R.color.white)
                ) {
                    SurfaceCard()
                }
            }
        }
    }
}

var surfaceView: WGPUSurfaceView? = null

@Composable
fun SurfaceCard() {
    var selected by remember { mutableStateOf("boids") }
    val toggleValues = listOf("boids", "MSAA line", "cube", "water", "shadow", "HDR ASTC")
    val screenWidth = LocalConfiguration.current.screenWidthDp.dp
    Column(modifier = Modifier.fillMaxSize()) {
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.Center,
            modifier = Modifier
                .height(44.dp)
                .padding(horizontal = 0.dp, vertical = 7.dp)
                .fillMaxWidth()
        ) {
            Text(text = "wgpu in Android App", fontSize = 20.sp, fontWeight = FontWeight.Bold)
        }
        Row(
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.Center,
            modifier = Modifier
                .height(54.dp)
                .padding(horizontal = 0.dp, vertical = 9.dp)
                .fillMaxWidth()
        ) {
            ToggleButton(
                currentSelection = selected,
                toggleStates = toggleValues,
                onToggleChange = { title ->
                    selected = title
                    toggleValues.forEachIndexed { idx, v ->
                        if (v == title) {
                            surfaceView?.changeExample(idx)
                        }
                    }
                },
            )

        }
        Spacer(modifier = Modifier.height(8.dp))
        AndroidView(
            factory = { ctx ->
                val sv = WGPUSurfaceView(context = ctx)
                surfaceView = sv
                sv
            },
            modifier = Modifier
                .fillMaxWidth()
                .height(screenWidth),
        )
    }
}
