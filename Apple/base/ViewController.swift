//
//  ViewController.swift
//
//  Created by LiJinlei on 2021/9/10.
//

import UIKit

class ViewController: UIViewController {
    @IBOutlet private var metalV: MetalView!
    private var wgpuCanvas: OpaquePointer?
    
    private lazy var displayLink: CADisplayLink = {
        CADisplayLink(target: self, selector: #selector(enterFrame))
    }()
    
    override func viewDidLoad() {
        super.viewDidLoad()
       
        self.displayLink.add(to: .current, forMode: .default)
        self.displayLink.isPaused = true
    }
    
    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        self.view.backgroundColor = .white
        self.setupWGPUCanvasIfNeeded()
        self.displayLink.isPaused = false
    }
    
    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        self.displayLink.isPaused = true
    }
    
    @objc private func enterFrame() {
        guard let canvas = self.wgpuCanvas else { return }
        enter_frame(canvas)
    }
    
    @IBAction private func changeExample(sender: UISegmentedControl) {
        guard let canvas = self.wgpuCanvas else { return }
        let index = sender.selectedSegmentIndex == 2 ? 5 : sender.selectedSegmentIndex
        change_example(canvas, Int32(index))
    }

    private func setupWGPUCanvasIfNeeded() {
        guard self.wgpuCanvas == nil else { return }
        
        let viewPointer = Unmanaged.passUnretained(self.metalV).toOpaque()
        let metalLayer = Unmanaged.passUnretained(self.metalV.layer).toOpaque()
        let maximumFrames = Int32(UIScreen.main.maximumFramesPerSecond)
        
        let viewObj = ios_view_obj_t(
            view: viewPointer,
            metal_layer: metalLayer,
            maximum_frames: maximumFrames,
            callback_to_swift: callback_to_swift
        )
        
        self.wgpuCanvas = create_wgpu_canvas(viewObj)
    }
}

func callback_to_swift(arg: Int32) {
    DispatchQueue.main.async {
        switch arg {
        case 0:
            print("wgpu canvas created!")
        case 1:
            print("canvas enter frame")
        default:
            break
        }
    }
}
