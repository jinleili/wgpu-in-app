//
//  ViewController.swift
//  wgpu_test
//
//  Created by LiJinlei on 2021/9/10.
//

import UIKit

class ViewController: UIViewController {
    @IBOutlet var metalV: MetalView!
    var wgpuCanvas: OpaquePointer?

    override func viewDidLoad() {
        super.viewDidLoad()
       
        let viewPointer = UnsafeMutableRawPointer(Unmanaged.passRetained(self.metalV).toOpaque())
        let metalLayer = UnsafeMutableRawPointer(Unmanaged.passRetained(self.metalV.layer).toOpaque())
       

        let maximumFrames = UIScreen.main.maximumFramesPerSecond
        let viewObj = ios_view_obj(view: viewPointer, metal_layer: metalLayer,maximum_frames: Int32(maximumFrames), callback_to_swift: callback_to_swift)
        
        wgpuCanvas = create_wgpu_canvas(viewObj)
    }

}

func callback_to_swift(arg: Int32) {
    DispatchQueue.main.async {
        switch arg {
        case 123:
            print("wgpu enter frame")
            break
            
        default:
            break
        }
    }
    
}
