from __future__ import annotations

import asyncio

from openhub_bo.core import PaymentStatus


def test_async_flow_shares_one_token(mock, qr_args):
    async def scenario() -> None:
        qr_client = mock.async_client()
        async with qr_client.session:
            qrs = await asyncio.gather(
                *(qr_client.generate_qr(**{**qr_args, "reference": f"9000{i}"}) for i in range(5))
            )
            mock.pay(qrs[0].reference)
            statuses = await asyncio.gather(*(qr_client.get_qr_status(q.reference) for q in qrs))
        assert statuses[0].status is PaymentStatus.PAID
        assert all(s.status is PaymentStatus.PENDING for s in statuses[1:])

    asyncio.run(scenario())
    assert mock.token_requests == 1


def test_async_refreshes_revoked_token(mock, qr_args):
    async def scenario() -> PaymentStatus:
        qr_client = mock.async_client()
        qr = await qr_client.generate_qr(**qr_args)
        mock.revoke_tokens()
        return (await qr_client.get_qr_status(qr.reference)).status

    assert asyncio.run(scenario()) is PaymentStatus.PENDING
    assert mock.token_requests == 2


def test_async_session_survives_event_loops(mock, qr_args):
    qr_client = mock.async_client()
    first = asyncio.run(qr_client.generate_qr(**qr_args))
    # A second asyncio.run uses a new loop; the lock is created lazily per session.
    assert asyncio.run(qr_client.get_qr_status(first.reference)).status is PaymentStatus.PENDING


def test_async_cancel(mock, qr_args):
    async def scenario() -> PaymentStatus:
        qr_client = mock.async_client()
        qr = await qr_client.generate_qr(**qr_args)
        await qr_client.cancel_qr(qr.reference)
        return (await qr_client.get_qr_status(qr.reference)).status

    assert asyncio.run(scenario()) is PaymentStatus.CANCELLED
