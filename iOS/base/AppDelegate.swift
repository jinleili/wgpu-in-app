//
//  AppDelegate.swift
//
//  Created by LiJinlei on 2021/9/10.
//

import UIKit

@main
class AppDelegate: UIResponder, UIApplicationDelegate {
    var window: UIWindow?
        
    func application(_ application: UIApplication, didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?) -> Bool {
        window = UIWindow(frame: UIScreen.main.bounds)
        let mainStroryBoard = UIStoryboard(name: "Main", bundle: nil)
        window?.rootViewController = mainStroryBoard.instantiateInitialViewController()

        window?.makeKeyAndVisible()
        return true
    }

}

