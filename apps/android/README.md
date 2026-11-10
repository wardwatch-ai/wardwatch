# Wardwatch for Android

A native Kotlin + Jetpack Compose app. App ID `ai.wardwatch.mobile`.

What it does today, all on the phone (nothing is sent anywhere yet):

- Shows the phone's maker, model, Android version and security patch level.
- Nudges system updates by the patch's age: current up to 45 days, due after 45, and the amber act-now nudge after 90, since by then it has missed several monthly security bulletins. The button opens the system update screen.
- Lists the apps you installed or updated, with their versions. This needs `QUERY_ALL_PACKAGES`, which Google Play allows only after approving a declaration for the app.

The screen is a placeholder until the mobile design is approved. Brand colors live in `Theme.kt` and a unit test keeps them in step with `packages/ui/src/tokens/brand.json`.

## Build

You need JDK 21 and the Android SDK (`ANDROID_HOME`, or `sdk.dir` in `local.properties`).

```sh
./gradlew test lintDebug assembleDebug assembleRelease
```

The release APK is unsigned (`app/build/outputs/apk/release/app-release-unsigned.apk`). Signing and Play uploads are done by hand under Matthew Ward's developer account, never by CI.
