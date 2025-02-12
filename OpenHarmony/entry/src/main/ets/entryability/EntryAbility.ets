import { RustAbility } from '@ohos-rs/ability'
import Want from '@ohos.app.ability.Want'
import { AbilityConstant } from '@kit.AbilityKit';

export default class EntryAbility extends RustAbility {
  public moduleName: string = "wgpu_in_app"

  async onCreate(want: Want, launchParam: AbilityConstant.LaunchParam): Promise<void> {
    super.onCreate(want, launchParam);
  }
}
