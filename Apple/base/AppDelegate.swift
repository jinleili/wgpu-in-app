//
//  AppDelegate.swift
//
//  Created by LiJinlei on 2021/9/10.
//

import UIKit
import ObjectiveC

@main
class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
        
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        // [wgpu v29 workaround]: Apple 在 iOS 上的 CaptureMTLDevice (开启 GPU Frame Capture 时) 
        // 漏掉了某些特性的实现，导致 wgpu 初始化查询时崩溃。我们在此动态返回默认值。
        if let captureDeviceClass = NSClassFromString("CaptureMTLDevice") {
            let blockFalse: @convention(block) (Any) -> Bool = { _ in false }
            let impFalse = imp_implementationWithBlock(blockFalse)
            
            // Fix 1: supports32BitFloatFiltering
            let selFloat = NSSelectorFromString("supports32BitFloatFiltering")
            if !class_respondsToSelector(captureDeviceClass, selFloat) {
                class_addMethod(captureDeviceClass, selFloat, impFalse, "B@:")
            }
            
            // Fix 2: hasUnifiedMemory
            let selUnified = NSSelectorFromString("hasUnifiedMemory")
            if !class_respondsToSelector(captureDeviceClass, selUnified) {
                // iOS 设备基本都是统一内存，但这里为了安全先返回 true 试试（大部分 iOS App 预期这是 true）
                let blockTrue: @convention(block) (Any) -> Bool = { _ in true }
                let impTrue = imp_implementationWithBlock(blockTrue)
                class_addMethod(captureDeviceClass, selUnified, impTrue, "B@:")
            }
            
            // Fix 3: supportsRaytracing
            let selRaytracing = NSSelectorFromString("supportsRaytracing")
            if !class_respondsToSelector(captureDeviceClass, selRaytracing) {
                let blockFalseRT: @convention(block) (Any) -> Bool = { _ in false }
                let impFalseRT = imp_implementationWithBlock(blockFalseRT)
                class_addMethod(captureDeviceClass, selRaytracing, impFalseRT, "B@:")
            }
        }

        window = UIWindow(frame: UIScreen.main.bounds)
        let mainStroryBoard = UIStoryboard(name: "Main", bundle: nil)
        window?.rootViewController = mainStroryBoard.instantiateInitialViewController()

        window?.makeKeyAndVisible()
        return true
    }

}

