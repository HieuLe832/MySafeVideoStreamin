import React from 'react';
import { HardDrive } from 'lucide-react';
import { formatBytes } from '../utils/formatters';

interface QuotaIndicatorProps {
  totalUsedBytes: number;
  maxLimitBytes: number;
}

export const QuotaIndicator: React.FC<QuotaIndicatorProps> = ({
  totalUsedBytes,
  maxLimitBytes,
}) => {
  const getQuotaPercentage = () => {
    if (maxLimitBytes === 0) return 0;
    return Math.min(100, Math.round((totalUsedBytes * 100) / maxLimitBytes));
  };

  const quotaPercent = getQuotaPercentage();

  const getQuotaClass = () => {
    if (quotaPercent >= 90) return 'danger';
    if (quotaPercent >= 70) return 'warning';
    return '';
  };

  return (
    <div className="quota-banner" id="storage-quota-banner">
      <div className="quota-info">
        <div className="quota-title">
          <HardDrive size={18} className="gradient-text" />
          <span>Giới hạn bộ nhớ lưu trữ Cloud R2 (Best Effort)</span>
        </div>
        <span className="quota-value">
          {formatBytes(totalUsedBytes)} / {formatBytes(maxLimitBytes)} ({quotaPercent}%)
        </span>
      </div>
      <div className="quota-bar-bg">
        <div
          className={`quota-bar-fill ${getQuotaClass()}`}
          style={{ width: `${quotaPercent}%` }}
        ></div>
      </div>
    </div>
  );
};
