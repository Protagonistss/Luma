import { LumaBrandMark } from "@/shared/icons";

interface SplashScreenProps {
  exiting?: boolean;
}

export function SplashScreen({ exiting = false }: SplashScreenProps) {
  return (
    <div
      className={`splash-screen ${exiting ? "splash-screen--exit" : ""}`}
      role="status"
      aria-label="Luma 正在启动"
    >
      <div className="splash-screen__content">
        <LumaBrandMark size={72} animated />
        <p className="splash-screen__title">Luma</p>
      </div>
    </div>
  );
}
