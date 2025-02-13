import { DefaultXComponent } from "@ohos-rs/ability"
import { ItemRestriction, SegmentButton, SegmentButtonOptions, SegmentButtonTextItem } from '@kit.ArkUI';
import { changeRender } from "libwgpu_in_app.so"

@Entry
@Component
struct Index {
  @State tabOptions: SegmentButtonOptions = SegmentButtonOptions.capsule({
    buttons: [{ text: 'boids' },
      { text: 'MSAA line' },
      { text: 'cube' },
      { text: "water" },
      { text: "shadow" }] as ItemRestriction<SegmentButtonTextItem>,
    backgroundBlurStyle: BlurStyle.BACKGROUND_THICK,
  })
  @State @Watch("handleChange") tabSelectedIndexes: number[] = [0]

  handleChange() {
    changeRender(this.tabSelectedIndexes[0])
  }

  build() {
    Row() {
      Column() {
        SegmentButton({ options: this.tabOptions, selectedIndexes: $tabSelectedIndexes })
        DefaultXComponent()
      }
      .width('100%')
    }
    .height('100%')
  }
}
