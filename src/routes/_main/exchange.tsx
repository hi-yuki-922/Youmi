import { createFileRoute } from "@tanstack/react-router";
import { CircleDollarSign, RefreshCw, X } from "lucide-react";
import { useEffect, useState } from "react";

import { CurrencyPicker } from "@/components/CurrencyPicker";
import { Button } from "@/components/ui/Button";
import { FeedbackStrip } from "@/components/ui/FeedbackStrip";
import { Input } from "@/components/ui/Input";
import { ensureRates, fetchRates } from "@/lib/exchange";
import {
  restoreOneOf,
  restoreString,
  restoreStringArray,
  usePersistedToolState,
} from "@/lib/tool-state";
import {
  convert,
  CURRENCIES,
  currencyName,
  formatAmount,
  formatUpdatedAt,
  type RateSnapshot,
} from "@/tools/exchange/core";

const restoreCurrencyCode = restoreOneOf(CURRENCIES.map((currency) => currency.code));

const restoreTargets = restoreStringArray(CURRENCIES.map((currency) => currency.code));

const ExchangePage = () => {
  const [source, setSource] = usePersistedToolState("exchange.source", "CNY", restoreCurrencyCode);
  const [targets, setTargets] = usePersistedToolState<string[]>(
    "exchange.targets",
    ["USD", "EUR", "JPY"],
    restoreTargets,
  );
  const [amount, setAmount] = usePersistedToolState("exchange.amount", "1", restoreString);
  const [snapshot, setSnapshot] = useState<RateSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [refreshing, setRefreshing] = useState(false);

  // 挂载即保数据可用：缓存新鲜直接用（即时呈现），过期/缺失后台拉取
  useEffect(() => {
    let active = true;
    void ensureRates().then((availability) => {
      if (!active) return;
      setSnapshot(availability.snapshot);
      setError(availability.error);
    });
    return () => {
      active = false;
    };
  }, []);

  const refresh = () => {
    setRefreshing(true);
    void fetchRates()
      .then((fresh) => {
        setSnapshot(fresh);
        setError(null);
      })
      .catch((cause) => {
        console.error("手动刷新汇率失败", cause);
        setError("刷新失败，当前展示离线缓存");
      })
      .finally(() => setRefreshing(false));
  };

  const amountNumber = Number(amount.replace(/,/g, ""));
  const amountInvalid = amount.trim() === "" || !Number.isFinite(amountNumber);

  const addTarget = (code: string) => {
    if (!targets.includes(code)) setTargets([...targets, code]);
  };
  const removeTarget = (code: string) => {
    setTargets(targets.filter((target) => target !== code));
  };

  return (
    <div className="flex h-full min-h-0 flex-col gap-4 p-6">
      <div className="flex items-center justify-between gap-3">
        <h1 className="text-xl font-semibold">汇率</h1>
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          {snapshot && <span>数据更新于 {formatUpdatedAt(snapshot.updatedAt)}</span>}
          <Button
            variant="ghost"
            size="icon"
            aria-label="刷新汇率"
            title="刷新汇率"
            onClick={refresh}
            disabled={refreshing}
            className="size-8 text-muted-foreground"
          >
            <RefreshCw className={refreshing ? "animate-spin" : undefined} />
          </Button>
        </div>
      </div>

      {error && <FeedbackStrip kind="error">{error}</FeedbackStrip>}

      <div className="flex flex-wrap items-center gap-2">
        <CurrencyPicker
          value={source}
          options={CURRENCIES}
          onChange={setSource}
          ariaLabel="源币种"
        />
        <span className="text-sm text-muted-foreground">{currencyName(source)}</span>
        <Input
          aria-label="金额"
          value={amount}
          onChange={(event) => setAmount(event.target.value)}
          inputMode="decimal"
          placeholder="输入金额"
          className="h-8 w-44 font-mono"
        />
      </div>

      <ul
        className="flex min-h-0 flex-1 flex-col gap-1.5 overflow-y-auto"
        aria-label="目标币种换算结果"
      >
        {targets.map((target) => {
          const result =
            amountInvalid || snapshot === null
              ? null
              : convert(amountNumber, source, target, snapshot.rates);
          return (
            <li
              key={target}
              className="flex items-center gap-3 rounded-md border px-3 py-2 text-sm"
            >
              <CurrencyPicker
                value={target}
                options={CURRENCIES}
                onChange={(code) => {
                  // 与源币种互换：换方向最常用的路径
                  if (code === source) setSource(target);
                  setTargets(targets.map((existing) => (existing === target ? code : existing)));
                }}
                ariaLabel={`目标币种 ${target}`}
              />
              <span className="min-w-0 flex-1 truncate text-muted-foreground">
                {currencyName(target)}
              </span>
              <span className="shrink-0 font-mono tabular-nums" aria-live="polite">
                {result === null ? "—" : result.ok ? formatAmount(result.value) : result.message}
              </span>
              <Button
                variant="ghost"
                size="icon"
                aria-label={`移除 ${currencyName(target)}`}
                onClick={() => removeTarget(target)}
                className="size-7 shrink-0 text-muted-foreground"
              >
                <X />
              </Button>
            </li>
          );
        })}
        <li>
          <CurrencyPicker
            value=""
            options={CURRENCIES}
            onChange={addTarget}
            addLabel="添加币种…"
            ariaLabel="添加目标币种"
          />
        </li>
      </ul>

      {snapshot === null && (
        <p className="flex items-center gap-2 text-sm text-muted-foreground">
          <CircleDollarSign className="size-4" />
          正在获取汇率数据…
        </p>
      )}
    </div>
  );
};

export const Route = createFileRoute("/_main/exchange")({
  component: ExchangePage,
});
