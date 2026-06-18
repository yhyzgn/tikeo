package com.example;

import com.xxl.job.core.handler.annotation.XxlJob;

class BillingJob {
    @XxlJob("billingProcessor")
    public void execute() {
    }
}
