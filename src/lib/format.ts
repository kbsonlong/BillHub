export function money(amount: number): string {
  const sign = amount < 0 ? "-" : "";
  return `${sign}¥${(Math.abs(amount) / 100).toLocaleString("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })}`;
}

export function dateTime(seconds: number | null): string {
  if (!seconds) return "—";
  return new Date(seconds * 1000).toLocaleString("zh-CN", {
    timeZone: "Asia/Shanghai",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

export const labels: Record<string, string> = {
  wechat: "微信",
  alipay: "支付宝",
  bank: "银行卡",
  payment: "支付",
  refund: "退款",
  transfer: "转账",
  top_up: "充值",
  withdrawal: "提现",
  adjustment: "调整",
  expense: "支出",
  income: "收入",
  neutral: "中性",
  pending: "待确认",
  settled: "已入账",
  closed: "已关闭",
  reversed: "已反转",
  unknown: "待核实",
};
