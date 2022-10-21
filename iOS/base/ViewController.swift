//
//  ViewController.swift
//
//  Created by LiJinlei on 2021/9/10.
//

import UIKit

class ViewController: UIViewController {
    @IBOutlet var metalV: MetalView!
    var wgpuCanvas: OpaquePointer?
    
    lazy var displayLink: CADisplayLink = {
        CADisplayLink.init(target: self, selector: #selector(enterFrame))
    }()
    
    override func viewDidLoad() {
        super.viewDidLoad()
       
        self.displayLink.add(to: .current, forMode: .default)
        self.displayLink.isPaused = true
    }
    
    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        self.view.backgroundColor = .white
        if wgpuCanvas == nil {
            let viewPointer = Unmanaged.passUnretained(self.metalV).toOpaque()
            let metalLayer = Unmanaged.passUnretained(self.metalV.layer).toOpaque()
            let maximumFrames = UIScreen.main.maximumFramesPerSecond
            
            let viewObj = ios_view_obj(view: viewPointer, metal_layer: metalLayer,maximum_frames: Int32(maximumFrames), callback_to_swift: callback_to_swift)
            
            wgpuCanvas = create_wgpu_canvas(viewObj)
        }
        self.displayLink.isPaused = false
    }
    
    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        displayLink.isPaused = true
    }
    
    @objc func enterFrame() {
        guard let canvas = self.wgpuCanvas else {
            return
        }
        // call rust
        enter_frame(canvas)
    }
    
    @IBAction func changeExample(sender: UISegmentedControl) {
        guard let canvas = self.wgpuCanvas else {
            return
        }
        var index = sender.selectedSegmentIndex
        if index == 2 {
            index = 5
        }
        change_example(canvas, Int32(index))
    }

}

func callback_to_swift(arg: Int32) {
    DispatchQueue.main.async {
        switch arg {
        case 0:
            print("wgpu canvas created!")
            break
        case 1:
            print("canvas enter frame")
            break
            
        default:
            break
        }
    }
    
}
