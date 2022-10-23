package name.jinleili.wgpu

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.selection.toggleable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.Divider
import androidx.compose.material.MaterialTheme
import androidx.compose.material.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp

@Composable
fun ToggleButton(
    currentSelection: String,
    toggleStates: List<String>,
    onToggleChange: (String) -> Unit
) {
    val selectedTint = MaterialTheme.colors.primary
    val unselectedTint = Color.Unspecified
    Row(
        modifier = Modifier
            .height(IntrinsicSize.Min)
            .background(color = Color.LightGray, shape = RoundedCornerShape(6.dp))
    ) {
        toggleStates.forEachIndexed { index, toggleState ->
            val isSelected = currentSelection.lowercase() == toggleState.lowercase()
            val backgroundTint = if (isSelected) selectedTint else unselectedTint
            val textColor = if (isSelected) Color.White else Color.Unspecified

            if (index != 0) {
                Divider(
                    color = Color(0x55666666),
                    modifier = Modifier
                        .fillMaxHeight()
                        .padding(horizontal = 0.dp, vertical = 7.dp)
                        .width(1.dp)
                )
            }

            Row(
                modifier = Modifier
                    .background(color = backgroundTint, shape = RoundedCornerShape(6.dp))
                    .toggleable(
                        value = isSelected,
                        enabled = true,
                        onValueChange = { _ ->
//                            if (selected) {
                                onToggleChange(toggleState)
//                            }
                        })
            ) {
                Text(toggleState, color = textColor, modifier = Modifier.padding(vertical = 2.dp, horizontal = 4.dp))
            }

        }
    }
}
